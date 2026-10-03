"""Recover auditable proposals from a response without repairing or adopting it."""
import hashlib
import json
import math
from .io_utils import digest


def strict_json(text):
    def pairs(items):
        out={}
        for k,v in items:
            if k in out:raise ValueError('duplicate JSON key: '+k)
            out[k]=v
        return out
    def nonfinite(value):raise ValueError('non-finite JSON number: '+value)
    def finite_float(value):
        parsed=float(value)
        if not math.isfinite(parsed):nonfinite(value)
        return parsed
    return json.loads(text,object_pairs_hook=pairs,parse_constant=nonfinite,parse_float=finite_float)


def unexpected_fields(raw, parsed, prefix=''):
    """Do not silently discard fields which permissive historical models ignore."""
    issues=[]
    if isinstance(raw,dict) and isinstance(parsed,dict):
        for key,value in raw.items():
            path=f'{prefix}.{key}' if prefix else key
            if key not in parsed:issues.append('unexpected field: '+path)
            else:issues.extend(unexpected_fields(value,parsed[key],path))
    elif isinstance(raw,list) and isinstance(parsed,list):
        for i,(a,b) in enumerate(zip(raw,parsed)):issues.extend(unexpected_fields(a,b,f'{prefix}[{i}]'))
    return issues


def inspect_response(text, segments):
    """Schema validity, provenance and semantic diagnostics remain separate.

    Segments contain exact text and source offsets; matching across concatenated
    excerpts is forbidden. Nothing here writes a Claim or changes model output.
    """
    from .llm import ExtractedClaim, ResearchGap, ResearchConflict, _time_value
    from .time_interpretation import native_numbers_match
    result={'version':1,'raw_sha256':hashlib.sha256(text.encode()).hexdigest(),
            'status':'unreadable','claims':[],'gaps':[],'conflicts':[],
            'envelope_errors':[],'scope':'Recovered proposals only; no semantic endorsement or scientific import.'}
    try:
        data=strict_json(text)
        if not isinstance(data,dict):raise ValueError('response must be an object')
    except (ValueError,TypeError) as exc:
        result['envelope_errors']=[str(exc)];return result
    for key in data.keys()-{'claims','gaps','conflicts','paper_summary'}:
        result['envelope_errors'].append('unexpected envelope field: '+key)
    if not isinstance(data.get('paper_summary'),str):result['envelope_errors'].append('missing/invalid paper_summary')
    result['paper_summary']=data.get('paper_summary')
    for field,model in [('claims',ExtractedClaim),('gaps',ResearchGap),('conflicts',ResearchConflict)]:
        values=data.get(field)
        if not isinstance(values,list):
            result['envelope_errors'].append('missing/invalid '+field);continue
        for index,raw in enumerate(values):
            row={'index':index,'raw':raw,'raw_digest':digest(raw),'schema_valid':False}
            try:
                parsed=model.model_validate(raw,strict=True)
                obj=parsed.model_dump(mode='json')
                extras=unexpected_fields(raw,obj)
                if extras:raise ValueError('; '.join(extras))
                if field=='claims' and parsed.time.source_values is None:
                    raise ValueError('native source_values required')
                row.update(schema_valid=True,proposal=obj)
            except ValueError as exc:
                row['errors']=str(exc)
                result[field].append(row);continue
            if field=='claims':
                matches=[]
                if len(parsed.quote.strip())>=20:
                    for segment in segments:
                        offset=segment['text'].find(parsed.quote)
                        while offset>=0:
                            start=segment['start']+offset;end=start+len(parsed.quote)
                            # Context and overlap can repeat a source span.
                            match={'source_id':segment['source_id'],'text_sha256':segment['text_sha256'],
                                   'start':start,'end':end,'purpose':segment['purpose']}
                            if match not in matches:matches.append(match)
                            offset=segment['text'].find(parsed.quote,offset+1)
                native=parsed.time.source_values.model_dump(exclude_none=True)
                row.update(spans=matches,exact_quote=bool(matches),
                           native_support=native_numbers_match(native,parsed.quote),
                           primary_evidence=any(s['purpose']=='primary' for s in matches),
                           semantic_status='pending',semantic_flags=[])
                try:row['calendar_projection']=_time_value(parsed.time).model_dump(mode='json')
                except ValueError as exc:row['normalization_error']=str(exc)
                projection=row.get('calendar_projection',{})
                row['calendar_conflicts']=[]
                for key in ('earliest','latest','mean','median','value','hpd95_low','hpd95_high'):
                    supplied=getattr(parsed.time,key)
                    if supplied is None:continue
                    if key.startswith('hpd95_'):
                        hpd=projection.get('hpd95')
                        computed=hpd[0 if key.endswith('low') else 1] if hpd else None
                    else:computed=projection.get(key)
                    if supplied!=computed:
                        row['calendar_conflicts'].append({'field':key,'model_value':supplied,'program_value':computed})
                if row['calendar_conflicts']:
                    row['semantic_flags'].append('model calendar fields conflict with native-value projection')
                if parsed.epistemic_role=='posterior' and parsed.subject_type=='sampled_clade' and parsed.event not in ('crown_divergence','stem_divergence','model_lineage_span'):
                    row['semantic_flags'].append('posterior sampled-clade event needs review')
                if parsed.epistemic_role=='calibration_prior' and parsed.event=='relative_constraint':
                    row['semantic_flags'].append('absolute calibration mislabeled relative constraint')
                if parsed.epistemic_role=='secondary_report' and not parsed.reported_from:
                    row['semantic_flags'].append('secondary report lacks attribution')
                if parsed.event=='model_lineage_span' and native['form'] not in ('duration','event_range'):
                    row['semantic_flags'].append('lineage span lacks two temporal endpoints')
                row['mechanically_ready']=bool(matches) and row['native_support'] and 'normalization_error' not in row and not row['calendar_conflicts']
            result[field].append(row)
    invalid=sum(not r['schema_valid'] for k in ('claims','gaps','conflicts') for r in result[k])
    result.update(status='partial' if invalid or result['envelope_errors'] else 'schema_valid',
                  invalid_items=invalid,valid_claims=sum(r['schema_valid'] for r in result['claims']))
    return result


def source_segment(text, source_id, purpose='primary', start=0, text_sha256=None):
    return {'source_id':source_id,'text_sha256':text_sha256 or hashlib.sha256(text.encode()).hexdigest(),
            'start':start,'end':start+len(text),'purpose':purpose,'text':text}
