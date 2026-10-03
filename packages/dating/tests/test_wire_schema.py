import copy,json,unittest
from unittest.mock import patch
from types import SimpleNamespace
from pydantic import BaseModel,Field
from harness.llm_backends import strict_schema,wire_schema,OpenRouterBackend
from harness.model_requests import freeze_request
from harness.compact_extraction import CompactExtraction
from harness.model_config import validate_config

class Collision(BaseModel):
    title:str=Field(min_length=5)
    maximum:list[int]=Field(max_length=2)

class WireTests(unittest.TestCase):
    def test_projection_preserves_property_names_and_canonical_validation(self):
        original=strict_schema(Collision);minimal=wire_schema(Collision,'minimal')
        self.assertEqual(set(minimal['properties']),{'title','maximum'})
        self.assertNotIn('minLength',minimal['properties']['title'])
        self.assertEqual(original,strict_schema(Collision))
        self.assertEqual(set(minimal['required']),{'title','maximum'})
        self.assertFalse(minimal['additionalProperties'])
        with self.assertRaises(ValueError):OpenRouterBackend.parse_response({'choices':[{'message':{'content':'{"title":"bad","maximum":[]}'}}]},Collision)
        with self.assertRaises(ValueError):wire_schema(Collision,'invented')

    def test_snapshot_retains_both_canonical_and_wire_contracts(self):
        cfg={'schema_version':2,'default_backend':'openrouter','backends':{'openrouter':{}},'roles':{'extract':{'backends':{'openrouter':{'model':'test','schema_profile':'minimal'}}}}}
        validate_config(cfg)
        with patch('harness.model_requests._save',side_effect=lambda kind,p:p):
            req=freeze_request('extract','f','i','s','u',CompactExtraction,SimpleNamespace(name='openrouter',cfg=cfg))
        self.assertEqual(req['schema'],strict_schema(CompactExtraction));self.assertEqual(req['wire_schema'],wire_schema(CompactExtraction,'minimal'))
        self.assertLess(len(json.dumps(req['wire_schema'])),len(json.dumps(req['schema'])))
        cfg['roles']['extract']['backends']['openrouter']['schema_profile']='typo'
        with self.assertRaises(ValueError):validate_config(cfg)
