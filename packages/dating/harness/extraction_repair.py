"""Explicit repair or abstention for each identified failed assertion."""
import hashlib
import json
from typing import Literal
from pydantic import BaseModel, ConfigDict, model_validator
from .llm import ExtractedClaim
from .extraction_recovery import strict_json, inspect_response


class RepairOutcome(BaseModel):
    model_config=ConfigDict(extra='forbid')
    original_index: int
    status: Literal['corrected','unresolved']
    claim: ExtractedClaim | None
    rationale: str

    @model_validator(mode='after')
    def consistent(self):
        if (self.status=='corrected')!=(self.claim is not None):
            raise ValueError('corrected requires a claim; unresolved requires null')
        return self


class RepairResponse(BaseModel):
    model_config=ConfigDict(extra='forbid')
    repairs: list[RepairOutcome]


def inspect_repairs(text, segments, targets):
    expected={t['index']:t['raw_digest'] for t in targets}
    result={'raw_sha256':hashlib.sha256(text.encode()).hexdigest(),'outcomes':[],
            'alignment_errors':[],'scope':'Proposed revisions or explicit abstention; no replacement or adoption.'}
    try:
        data=strict_json(text)
        if not isinstance(data,dict) or set(data)!={'repairs'} or not isinstance(data['repairs'],list):
            raise ValueError('repair response requires only a repairs list')
    except (ValueError,TypeError) as exc:
        result['alignment_errors'].append(str(exc));return result
    seen=set()
    for raw in data['repairs']:
        row={'raw':raw,'valid_outcome':False}
        try:
            parsed=RepairOutcome.model_validate(raw,strict=True)
            index=parsed.original_index
            if index not in expected or index in seen:raise ValueError('unknown or duplicate repair target')
            seen.add(index)
            row.update(valid_outcome=True,original_index=index,original_digest=expected[index],
                       status=parsed.status,rationale=parsed.rationale)
            if parsed.claim is not None:
                # Keep the original object so strict recovery can flag fields
                # which the historical claim models would otherwise ignore.
                wrapped=json.dumps({'claims':[raw['claim']],'paper_summary':'Proposed repair.','gaps':[],'conflicts':[]})
                row['claim_review']=inspect_response(wrapped,segments)['claims'][0]
        except ValueError as exc:row['error']=str(exc);result['alignment_errors'].append(str(exc))
        result['outcomes'].append(row)
    result['missing_indices']=sorted(set(expected)-seen)
    result['aligned']=not result['alignment_errors'] and not result['missing_indices']
    return result
