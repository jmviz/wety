"""Local, conditional contact scenarios; deliberately not registry relations.

Validation establishes references and consistency within each scenario. It does
not choose an account, prove a sound law, or unite alternative event graphs.
"""

from typing import Literal

from pydantic import Field

from .evidence import StrictModel


class ContactEvent(StrictModel):
    id: str = Field(min_length=1)
    description: str = Field(min_length=1)
    entity_scope: str = Field(min_length=1)


class ConditionalOrder(StrictModel):
    before: str
    after: str
    basis_findings: list[str] = Field(min_length=1)
    reasoning: str = Field(min_length=1)
    attribution: Literal["editorial_conditional_inference", "source_assertion"]


class ContactScenario(StrictModel):
    id: str = Field(min_length=1)
    description: str = Field(min_length=1)
    event_ids: list[str] = Field(min_length=1)
    basis_findings: list[str] = Field(min_length=1)
    conditions: list[str] = Field(min_length=1)
    orders: list[ConditionalOrder]
    unresolved: list[str] = Field(min_length=1)


class ContactCase(StrictModel):
    schema_version: Literal[1]
    id: str
    question: str = Field(min_length=1)
    evaluation_split: Literal["development"]
    historical_adjudication: Literal["unresolved"]
    export_policy: Literal["local_scenarios_only"]
    alternatives_exhaustive: Literal[False]
    alternatives_mutually_exclusive: Literal[False]
    events: list[ContactEvent] = Field(min_length=1)
    scenarios: list[ContactScenario] = Field(min_length=2)
    shared_limits: list[str] = Field(min_length=1)


def validate_contact_case(raw, finding_ids):
    try:
        case = ContactCase.model_validate(raw)
    except ValueError as exc:
        return ["invalid contact case: " + str(exc)]
    errors = []
    events = {e.id for e in case.events}
    if len(events) != len(case.events):
        errors.append("duplicate contact event")
    if len({s.id for s in case.scenarios}) != len(case.scenarios):
        errors.append("duplicate contact scenario")
    used = set()
    for scenario in case.scenarios:
        active = set(scenario.event_ids)
        used.update(active)
        if len(active) != len(scenario.event_ids) or not active <= events:
            errors.append("invalid scenario event inventory: " + scenario.id)
        refs = scenario.basis_findings + [f for o in scenario.orders for f in o.basis_findings]
        if not set(refs) <= set(finding_ids):
            errors.append("unknown scenario finding: " + scenario.id)
        edges = set()
        for order in scenario.orders:
            if order.before not in active or order.after not in active:
                errors.append("order endpoint outside scenario: " + scenario.id)
            edge = (order.before, order.after)
            if edge in edges:
                errors.append("duplicate scenario order: " + scenario.id)
            edges.add(edge)
        # Each scenario is checked separately: a union would add unsupported facts.
        remaining = set(active)
        while remaining:
            roots = {e for e in remaining if not any(b in remaining and a == e for b, a in edges)}
            if not roots:
                errors.append("conditional order cycle: " + scenario.id)
                break
            remaining -= roots
    if used != events:
        errors.append("unused contact event")
    return errors
