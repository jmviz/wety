import json
from pathlib import Path
import sys
import tempfile
import time
import unittest
from harness.evaluation_runtime import create_ledger,reserve,settle,BudgetExhausted,run_process_group
import subprocess

class RuntimeTests(unittest.TestCase):
    def test_deadline_and_backend_circuit_breaker_survive_new_plans(self):
        with tempfile.TemporaryDirectory() as tmp:
            p=Path(tmp)/'budget.json';policy={'max_calls':10,'cash_limit_usd':1,'reservation_usd':{'codex':0,'claude-agent':0},'max_consecutive_failures':2}
            create_ledger(p,policy)
            for i in range(2):
                key=reserve(p,{'v':i},'codex');settle(p,key,{'status':'timeout'})
            with self.assertRaisesRegex(BudgetExhausted,'consecutive'):reserve(p,{'new_plan':True},'codex')
            reserve(p,{'other_backend':True},'claude-agent')
            q=Path(tmp)/'expired.json';create_ledger(q,{**policy,'deadline_utc':'2000-01-01T00:00:00+00:00'})
            with self.assertRaisesRegex(BudgetExhausted,'deadline'):reserve(q,{},'codex')

    def test_concurrent_callers_cannot_overbook(self):
        from concurrent.futures import ThreadPoolExecutor
        with tempfile.TemporaryDirectory() as tmp:
            p=Path(tmp)/'budget.json';create_ledger(p,{'max_calls':20,'cash_limit_usd':1,'reservation_usd':{'openrouter':1}})
            def attempt(i):
                try:reserve(p,{'cell':i},'openrouter');return True
                except BudgetExhausted:return False
            with ThreadPoolExecutor(max_workers=4) as pool:self.assertEqual(sum(pool.map(attempt,range(8))),1)
            self.assertEqual(len(json.loads(p.read_text())['entries']),1)

    def test_shared_budget_survives_new_directories_unknown_cost_and_restarts(self):
        with tempfile.TemporaryDirectory() as tmp:
            p=Path(tmp)/'budget.json';policy={'max_calls':4,'cash_limit_usd':2,'reservation_usd':{'openrouter':1,'codex':0}}
            create_ledger(p,policy)
            a=reserve(p,{'root':'v1'},'openrouter');settle(p,a,{'status':'failed'})
            b=reserve(p,{'root':'v2'},'openrouter')
            with self.assertRaises(BudgetExhausted):reserve(p,{'root':'v3'},'openrouter')
            settle(p,b,{'status':'completed','execution':{'cost_usd':.2}})
            create_ledger(p,policy)
            with self.assertRaises(BudgetExhausted):reserve(p,{'root':'v1'},'openrouter')
            c=reserve(p,{'root':'subscription'},'codex');settle(p,c,{'status':'completed','execution':{'cost_usd':500}})
            entries=json.loads(p.read_text())['entries'];self.assertEqual(entries[a]['accounted_usd'],1)
            self.assertEqual(entries[c]['accounted_usd'],0)
            with self.assertRaises(ValueError):create_ledger(p,{**policy,'cash_limit_usd':100})

    def test_provider_overshoot_blocks_future_calls(self):
        with tempfile.TemporaryDirectory() as tmp:
            p=Path(tmp)/'budget.json';create_ledger(p,{'max_calls':10,'cash_limit_usd':2,'reservation_usd':{'openrouter':1}})
            key=reserve(p,{},'openrouter');settle(p,key,{'status':'completed','execution':{'cost_usd':3}})
            with self.assertRaises(BudgetExhausted):reserve(p,{'next':True},'openrouter')

    def test_timeout_kills_descendant_which_ignores_term(self):
        with tempfile.TemporaryDirectory() as tmp:
            marker=Path(tmp)/'should-not-exist'
            child="import signal,time,pathlib;signal.signal(signal.SIGTERM,signal.SIG_IGN);time.sleep(1.5);pathlib.Path("+repr(str(marker))+").touch()"
            parent="import subprocess,sys,time;subprocess.Popen([sys.executable,'-c',"+repr(child)+"]);time.sleep(10)"
            with self.assertRaises(subprocess.TimeoutExpired):run_process_group([sys.executable,'-c',parent],.3)
            time.sleep(.5)
            self.assertFalse(marker.exists())

    def test_normal_worker_exit_cleans_redirected_descendants(self):
        with tempfile.TemporaryDirectory() as tmp:
            marker=Path(tmp)/'orphan'
            child="import time,pathlib;time.sleep(.8);pathlib.Path("+repr(str(marker))+").touch()"
            parent="import subprocess,sys;subprocess.Popen([sys.executable,'-c',"+repr(child)+"],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)"
            result=run_process_group([sys.executable,'-c',parent],2)
            self.assertEqual(result.returncode,0)
            time.sleep(1)
            self.assertFalse(marker.exists())

if __name__=='__main__':unittest.main()
