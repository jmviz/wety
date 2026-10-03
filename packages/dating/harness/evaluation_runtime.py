"""Shared experiment reservations and bounded local process groups.

Reservations are conservative dispatch controls, not a provider billing guarantee.
Unknown/unfinished paid calls keep their reservation until explicitly reconciled.
"""

import json
import math
import os
from pathlib import Path
import signal
import subprocess
from datetime import datetime, timezone

from .config import now
from .io_utils import atomic_json, digest, locked


class BudgetExhausted(RuntimeError):
    pass


def create_ledger(path, policy):
    if policy.get("deadline_utc"):
        deadline = datetime.fromisoformat(policy["deadline_utc"])
        if deadline.tzinfo is None:
            raise ValueError("deadline requires an explicit timezone")
    if "max_consecutive_failures" in policy and (
        type(policy["max_consecutive_failures"]) is not int
        or policy["max_consecutive_failures"] < 1
    ):
        raise ValueError("positive consecutive failure limit required")
    if not isinstance(policy.get("max_calls"), int) or policy["max_calls"] <= 0:
        raise ValueError("positive logical-call limit required")
    amounts = [policy.get("cash_limit_usd"), *policy.get("reservation_usd", {}).values()]
    if any(not isinstance(x, (int, float)) or not math.isfinite(x) or x < 0 for x in amounts):
        raise ValueError("finite nonnegative budget amounts required")
    with locked(path):
        if Path(path).exists():
            if json.loads(Path(path).read_text())["policy"] != policy:
                raise ValueError(
                    "existing budget policy differs; do not silently reset expenditure"
                )
        else:
            atomic_json(path, {"version": 1, "policy": policy, "entries": {}, "created_at": now()})


def reserve(path, identity, backend):
    key = digest(identity)
    with locked(path):
        ledger = json.loads(Path(path).read_text())
        policy = ledger["policy"]
        if policy.get("deadline_utc") and datetime.now(timezone.utc) >= datetime.fromisoformat(
            policy["deadline_utc"]
        ):
            raise BudgetExhausted("shared experiment deadline reached")
        limit = policy.get("max_consecutive_failures")
        recent = sorted(
            (
                e
                for e in ledger["entries"].values()
                if e["backend"] == backend and "settled_at" in e
            ),
            key=lambda e: e["settled_at"],
            reverse=True,
        )
        if (
            limit
            and len(recent) >= limit
            and all(e.get("result_status") in ("failed", "timeout") for e in recent[:limit])
        ):
            raise BudgetExhausted("backend consecutive failure limit reached")
        if key in ledger["entries"]:
            raise BudgetExhausted(
                "cell already reserved or attempted; inspect retained result instead of redispatching"
            )
        if backend not in policy["reservation_usd"]:
            raise ValueError("backend has no declared reservation")
        amount = policy["reservation_usd"][backend]
        committed = sum(e["accounted_usd"] for e in ledger["entries"].values())
        if (
            len(ledger["entries"]) >= policy["max_calls"]
            or committed + amount > policy["cash_limit_usd"]
        ):
            raise BudgetExhausted("shared experiment call/cash reservation limit reached")
        ledger["entries"][key] = {
            "identity": identity,
            "backend": backend,
            "state": "in_flight",
            "reserved_usd": amount,
            "accounted_usd": amount,
            "reported_cash_usd": None,
            "reserved_at": now(),
        }
        atomic_json(path, ledger)
    return key


def remaining_seconds(path):
    policy = json.loads(Path(path).read_text())["policy"]
    if not policy.get("deadline_utc"):
        return float("inf")
    return max(
        0,
        (
            datetime.fromisoformat(policy["deadline_utc"]) - datetime.now(timezone.utc)
        ).total_seconds(),
    )


def settle(path, key, result):
    with locked(path):
        ledger = json.loads(Path(path).read_text())
        entry = ledger["entries"][key]
        cost = result.get("execution", {}).get("cost_usd")
        # Subscription SDK estimates must never release or consume a cash budget.
        known = (
            entry["backend"] == "openrouter"
            and result.get("status") == "completed"
            and isinstance(cost, (int, float))
            and math.isfinite(cost)
            and cost >= 0
        )
        entry.update(
            state="settled" if known or entry["reserved_usd"] == 0 else "uncertain",
            accounted_usd=cost if known else entry["reserved_usd"],
            reported_cash_usd=cost if known else None,
            reported_cost_usd=cost,
            result_status=result.get("status"),
            settled_at=now(),
        )
        atomic_json(path, ledger)


def run_process_group(command, timeout):
    """Own a fresh POSIX group; timeout also terminates its inherited descendants."""
    proc = subprocess.Popen(
        command, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, start_new_session=True
    )
    try:
        stdout, stderr = proc.communicate(timeout=timeout)
    except BaseException:
        try:
            os.killpg(proc.pid, signal.SIGTERM)
        except ProcessLookupError:
            pass
        try:
            proc.communicate(timeout=1)
        except subprocess.TimeoutExpired:
            pass
        # A parent may exit before a descendant which ignores TERM.
        try:
            os.killpg(proc.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        proc.communicate()
        raise
    # A backend may catch its own timeout and exit normally while grandchildren
    # continue with redirected pipes. They still belong to this owned group.
    try:
        os.killpg(proc.pid, signal.SIGKILL)
    except ProcessLookupError:
        pass
    return subprocess.CompletedProcess(command, proc.returncode, stdout, stderr)
