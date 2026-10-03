"""Source-native proposals and bounded semantic review; no scientific ingress.

The model never emits calendar conversions, catalog links, or editorial approval.
Historical Extraction readers remain unchanged.
"""
import hashlib
import json
import re
from typing import Literal
from pydantic import BaseModel,ConfigDict,Field,TypeAdapter,model_validator
from .llm import SourceTime,ExtractedClaim,ResearchGap,ResearchConflict
from .time_interpretation import BPConvention,UnresolvedCalendar
from .extraction_recovery import strict_json,unexpected_fields
from .io_utils import digest
from .section_packets import packet_prompt,verify_packet
from .time_interpretation import native_numbers_match


class Closed(BaseModel):
    model_config=ConfigDict(extra='forbid',allow_inf_nan=False)


class NativeBase(Closed):
    unit: SourceTime.model_fields['unit'].annotation
    present: int|None=Field(default=None,description=SourceTime.model_fields['present'].description)
    epoch_interpretation: BPConvention|None=Field(default=None,description=SourceTime.model_fields['epoch_interpretation'].description)
    age_scale: Literal['calendar','radiocarbon_uncalibrated','unknown']=Field(default='unknown',description=SourceTime.model_fields['age_scale'].description)
    precision: Literal['year','decade','century','millennium','unknown']='unknown'

    @model_validator(mode='after')
    def validate_native(self):
        if self.unit in ('BCE','CE','century_BCE','century_CE','millennium_BCE','millennium_CE'):
            for name in ('value','mean','median','lower','upper'):
                value=getattr(self,name,None)
                if value is not None and value<=0:
                    raise ValueError('native BCE/CE values must be positive; do not convert to astronomical years')
        SourceTime.model_validate(self.model_dump(exclude_none=True),strict=True)
        return self


class NativePoint(NativeBase):
    form: Literal['point']
    value: float


class NativeBound(NativeBase):
    form: Literal['bound']
    value: float
    relation: Literal['before','after','by','since']


class NativeRange(NativeBase):
    form: Literal['event_range','duration']
    lower: float
    upper: float|None
    ongoing: bool=False


class NativeDistribution(NativeBase):
    form: Literal['distribution']
    mean: float|None=None
    median: float|None=None
    lower: float|None=None
    upper: float|None=None
    interval_type: Literal['hpd','equal_tailed','reported_range','unknown']|None=None
    probability: float|None=None


class NativeUnknown(Closed):
    form: Literal['unknown']


class NativePrior(Closed):
    """Distribution parameters of a calibration prior, not posterior summaries."""
    form: Literal['prior_distribution']
    unit: Literal['BCE','CE','BP','ka_BP','century_BP','years_ago','ka_ago','astronomical']
    present: int|None=Field(default=None,description=SourceTime.model_fields['present'].description)
    epoch_interpretation: BPConvention|None=Field(default=None,description=SourceTime.model_fields['epoch_interpretation'].description)
    age_scale: Literal['calendar','radiocarbon_uncalibrated','unknown']='unknown'
    distribution_family: Literal['uniform','normal']
    location: float|None=Field(default=None,description='Underlying normal location parameter; not the mean of a truncated distribution.')
    standard_deviation: float|None=Field(default=None,gt=0,description='Underlying normal scale parameter in the native unit, before truncation.')
    support: Literal['bounded','truncated','unbounded']
    support_lower: float|None=None
    support_upper: float|None=None

    @model_validator(mode='after')
    def prior_shape(self):
        from .time_interpretation import calendar_epoch
        calendar_epoch(self.model_dump(exclude_none=True))
        if self.unit not in ('BP','ka_BP','century_BP','years_ago','ka_ago') and self.present is not None:
            raise ValueError('calendar-era prior does not use a present epoch')
        if self.support_lower is not None and self.support_upper is not None and self.support_lower>=self.support_upper:
            raise ValueError('prior support must have positive width')
        if self.support=='unbounded' and (self.support_lower is not None or self.support_upper is not None):
            raise ValueError('unbounded prior cannot have support endpoints')
        if self.support=='truncated' and self.support_lower is None and self.support_upper is None:
            raise ValueError('truncation requires at least one endpoint')
        if self.distribution_family=='uniform':
            if self.support!='bounded' or self.support_lower is None or self.support_upper is None or self.location is not None or self.standard_deviation is not None:
                raise ValueError('uniform prior requires bounded support and no normal parameters')
        elif self.location is None or self.standard_deviation is None or self.support=='bounded':
            raise ValueError('normal prior requires location/SD and truncated or unbounded support')
        # A normal location need not lie inside the truncation support. Do not
        # invent that restriction or silently calculate a truncated mean.
        if self.unit in ('BCE','CE') and any(v is not None and v<=0 for v in (self.location,self.support_lower,self.support_upper)):
            raise ValueError('native BCE/CE prior coordinates must be positive')
        return self


class NativeElapsed(Closed):
    form: Literal['elapsed_duration']
    unit: Literal['years','centuries','millennia']
    value: float=Field(ge=0)
    relation: Literal['exact','approximately','at_least','at_most','more_than','less_than']


class NativeOffset(Closed):
    form: Literal['relative_offset']
    unit: Literal['years','centuries','millennia']
    value: float=Field(ge=0)
    direction: Literal['before','after']
    approximate: bool
    reference_event: str=Field(min_length=1,description='Source-named anchor event, not a computed calendar date or another model assertion ID.')


RepresentedNativeTime = NativePoint|NativeBound|NativeRange|NativeDistribution|NativePrior|NativeElapsed|NativeOffset
NativeTime = RepresentedNativeTime|NativeUnknown


class Assertion(Closed):
    subject: str=Field(min_length=1,description='Literal dated subject, including artifacts or environment; never substitute the research family.')
    subject_type: Literal['sampled_clade','documentary_sample','reconstructed_stage','population','language_variety','artifact','environment','unresolved']
    event: ExtractedClaim.model_fields['event'].annotation
    role: ExtractedClaim.model_fields['epistemic_role'].annotation
    method: str
    native_time: NativeTime
    expression: str
    quote: str=Field(min_length=20,description='Exact source text including necessary qualifications; one contiguous span.')
    locator: str
    scenario: str|None=None
    reported_from: str|None=None
    qualifications: list[str]=Field(default_factory=list)
    relative_constraints: list[str]=Field(default_factory=list)
    sampled_taxa: list[str]=Field(default_factory=list)

    @model_validator(mode='after')
    def prior_role(self):
        if isinstance(self.native_time,NativePrior) and self.role not in ('calibration_prior','secondary_report'):
            raise ValueError('prior parameters require calibration_prior or an attributed secondary_report role')
        return self


class CompactExtraction(Closed):
    assertions: list[Assertion]=Field(max_length=12)
    gaps: list[ResearchGap]
    conflicts: list[ResearchConflict]
    overflow: bool=Field(description='True if the 12-assertion cap prevented complete extraction; describe omitted scope in gaps.')


def extraction_system(family):
    return ('Extract source-specific chronological proposals for research scope '+family+'. '
        'Return at most 12 assertions; if more exist set overflow true and describe omitted scope in gaps. '
        'Extract primary-segment evidence, using supplied context for meaning. No catalog IDs or computed calendar years. '
        'Native time only: preserve original units/statistic/epoch and bounds. BCE/CE numbers remain positive: 1200 BCE is value 1200, unit BCE. '
        'Represent explicit numeric dates whenever the schema supports them; missing epoch or age scale does not justify replacing the number with unknown. '
        'Keep unresolved epoch null and age_scale unknown, checking supplied methods context for the study epoch. Do not transfer that epoch to quoted secondary studies. '
        'BP conventionally means before 1950, but record an author-defined epoch where stated; distinguish calendar ages from uncalibrated radiocarbon ages. '
        'For a conventional BP1950 interpretation keep present null and use epoch_interpretation; present is only an explicitly source-stated epoch. Never set both. '
        'Coarse century/millennium dates require a range or bound, never point: mid-fifth century CE is event_range, unit century_CE, lower 5, upper 5, precision century, with mid-century retained in expression and qualifications. '
        'This enclosing range does not claim a more precise sub-century boundary. Lower/upper are numeric bounds in the original unit, including BCE. '
        'Unknown is for genuinely undated assertions or an unrepresentable shape; explain any schema gap. '
        'Use prior_distribution for uniform/normal calibration parameters: location and standard_deviation describe the underlying normal before truncation; support bounds are not HPDs. Do not replace a normal location with the mean of a truncated distribution. '
        'Use elapsed_duration for a length of time and relative_offset for an explicit quantified before/after relation to a source-named event. Neither is a years_ago date. The older duration shape is a span with dated endpoints, not an elapsed length. Keep approximate wording. '
        'Population movement, archaeological objects, environmental change, language divergence and documentary attestation are different subjects/events. '
        'Environment/artifact subjects may use event unspecified; do not force population_event. '
        'First surviving text is not language birth; sampled corpus dates need not be earliest attestation. '
        'Keep primary/secondary, priors/posteriors and scenario alternatives distinct. Name dependencies in reported_from and qualifications. '
        'Use relative_constraints and unknown time for purely relative arguments with no quantified offset. Unsupported distribution families, mixed-era endpoints and elapsed ranges remain explicit shape gaps. '
        'Do not infer century/millennium precision merely from trailing zeros or the word approximately; retain unknown precision unless the source specifies it. '
        'Copy exact contiguous quotes, preserving hedges and contested identification. Method and qualifications must agree with typed fields. '
        'A quoted historical hypothesis remains a secondary report with its hypothetical status retained; references are not independent confirmations. '
        'Do not merge different models or marginal intervals into a single historical chronology.')


def whitespace_index(text):
    """Only collapse whitespace, retaining the original Unicode text offsets."""
    tokens=list(re.finditer(r'\s+|\S',text))
    return ''.join(' ' if t[0].isspace() else t[0] for t in tokens),tokens


def quote_spans(quote,segments):
    spans=[]
    if len(quote.strip())<20:return spans
    for s in segments:
        offset=s['text'].find(quote)
        while offset>=0:
            row={k:s[k] for k in ('source_id','text_sha256','purpose')}
            row.update(start=s['start']+offset,end=s['start']+offset+len(quote),match='exact',source_quote=quote)
            if row not in spans:spans.append(row)
            offset=s['text'].find(quote,offset+1)
        if quote in s['text']:continue
        # Publisher NBSP/thin spaces and table line breaks often become ordinary
        # spaces in model output. Bind the actual contiguous source excerpt;
        # never rewrite the model quote, source text, punctuation or words.
        normalized,tokens=whitespace_index(s['text']);needle=re.sub(r'\s+',' ',quote)
        offset=normalized.find(needle)
        while offset>=0:
            a=tokens[offset].start();b=tokens[offset+len(needle)-1].end()
            row={k:s[k] for k in ('source_id','text_sha256','purpose')}
            row.update(start=s['start']+a,end=s['start']+b,match='whitespace_equivalent',source_quote=s['text'][a:b])
            if row not in spans:spans.append(row)
            offset=normalized.find(needle,offset+1)
    return spans


def inspect_compact(text,packet):
    verify_packet(packet)
    result={'raw_sha256':hashlib.sha256(text.encode()).hexdigest(),'status':'unreadable','claims':[],
            'gaps':[],'conflicts':[],'envelope_errors':[],'scope':'Strict source-native proposals; mechanical checks do not approve semantics.'}
    try:
        raw=strict_json(text)
        if not isinstance(raw,dict):raise ValueError('response must be an object')
    except (TypeError,ValueError) as exc:result['envelope_errors']=[str(exc)];return result
    if set(raw)!={'assertions','gaps','conflicts','overflow'}:result['envelope_errors'].append('missing or extra envelope fields')
    if type(raw.get('overflow')) is not bool:result['envelope_errors'].append('overflow must be explicit boolean')
    result['overflow']=raw.get('overflow')
    segments=[packet['primary'],*packet['context']]
    for field,model,out in [('assertions',Assertion,'claims'),('gaps',ResearchGap,'gaps'),('conflicts',ResearchConflict,'conflicts')]:
        values=raw.get(field)
        if not isinstance(values,list):result['envelope_errors'].append(field+' must be a list');continue
        if field=='assertions' and len(values)>12:result['envelope_errors'].append('assertion cap exceeded')
        for index,obj in enumerate(values):
            row={'index':index,'raw':obj,'raw_digest':digest(obj),'schema_valid':False}
            try:
                parsed=model.model_validate(obj,strict=True);proposal=parsed.model_dump(mode='json')
                extras=unexpected_fields(obj,proposal)
                if extras:raise ValueError('; '.join(extras))
                row.update(schema_valid=True,proposal=proposal)
            except (TypeError,ValueError) as exc:
                row['errors']=str(exc);result[out].append(row);continue
            if field=='assertions':
                native=parsed.native_time.model_dump(exclude_none=True);spans=quote_spans(parsed.quote,segments)
                row.update(spans=spans,exact_quote=any(s['match']=='exact' for s in spans),quote_supported=bool(spans),primary_evidence=any(s['purpose']=='primary' for s in spans),
                           native_support=native_numbers_match(native,parsed.quote),semantic_status='pending',semantic_flags=[])
                from .evidence import normalize_time
                try:
                    if native['form'] in ('prior_distribution','elapsed_duration','relative_offset'):
                        raise UnresolvedCalendar('Prior parameters, elapsed lengths and relative offsets are not calendar event intervals; no automatic projection.')
                    row['calendar_projection']=normalize_time(native).model_dump(mode='json')
                except UnresolvedCalendar as exc:row['calendar_abstention']=str(exc)
                except ValueError as exc:row['normalization_error']=str(exc)
                if parsed.role=='secondary_report' and not parsed.reported_from:row['semantic_flags'].append('secondary attribution missing')
                if native['form']=='unknown' and re.search(r'\d',parsed.expression):row['semantic_flags'].append('numeric expression with unknown native time requires review')
                if parsed.subject_type in ('environment','artifact') and parsed.event=='population_event':row['semantic_flags'].append('nonpopulation subject with population event')
                row['mechanically_ready']=bool(spans) and row['native_support'] and 'normalization_error' not in row
            result[out].append(row)
    invalid=sum(not x['schema_valid'] for name in ('claims','gaps','conflicts') for x in result[name])
    result.update(valid_claims=sum(x['schema_valid'] for x in result['claims']),invalid_items=invalid,
                  status='partial' if invalid or result['envelope_errors'] or result.get('overflow') else 'schema_valid')
    return result


class Judgment(Closed):
    assertion_id: str
    disposition: Literal['supported','revise','reject','unresolved']
    subject_event: Literal['supported','unsupported','uncertain']
    role_dependency: Literal['supported','unsupported','uncertain']
    native_time: Literal['supported','unsupported','uncertain']
    scenario_scope: Literal['supported','unsupported','uncertain']
    evidence_quote: str=Field(min_length=20)
    rationale: str
    @model_validator(mode='after')
    def check_support(self):
        if self.disposition=='supported' and any(getattr(self,k)!='supported' for k in ('subject_event','role_dependency','native_time','scenario_scope')):
            raise ValueError('supported disposition requires all semantic dimensions supported')
        return self


class SemanticReview(Closed):
    judgments: list[Judgment]=Field(max_length=12)
    omissions: list[str]=Field(description='Important source assertions absent from the extraction; not new adopted claims.')
    context_gaps: list[str]


def review_payload(task):
    return ('Review each proposed assertion against supplied original source context. Model proposals are untrusted interpretations, not evidence. '
            'Check subject/event, epistemic role and dependencies, native units/epoch/statistics/bound direction, scenario scope, and every qualification. '
            'Attestation does not date birth; environmental and population dates do not date language crowns. Priors are not results. '
            'A valid quote or matching number is not sufficient entailment. Use supported only if all dimensions are supported; '
            'Source fidelity is separate from historical truth: a faithfully qualified hypothesis can be supported. A null optional scenario label alone does not make scenario scope uncertain. '
            'Explicit numeric dates left solely in expression with native_time unknown need revision when representable, even if the epoch is unresolved. '
            'BCE/CE native values must be positive. Check methods context for author-defined epochs, without transferring them to cited studies. '
            'Use the supplied proposal schema to assess representability. Never request computed BCE/CE values for native years-ago dates. '
            'This schema cannot encode mixed-era endpoints, era-free ordinal periods, elapsed ranges or unsupported distribution families; preserve these in expression/constraints with a stated gap. '
            'A coarse enclosing century range with the narrower source wording retained in expression and qualifications is acceptable; do not invent precise early/middle/late boundaries. '
            'Use the dedicated prior_distribution, elapsed_duration and relative_offset shapes where supported. Prior support is not a posterior interval and a normal location/SD refers to the underlying normal before truncation. Do not convert elapsed lengths or offsets into absolute dates. '
            'Rounded digits or approximately alone do not establish century/millennium precision. '
            'revise/reject/unresolved are valid outcomes. Cite an exact source quote supporting your judgment. '
            'Return exactly one judgment per supplied assertion_id, retaining IDs; no rewriting or adopting claims. '
            'Report missed primary-segment assertions and missing context separately. Context-only assertions are not extraction omissions. '
            'For a repair batch, other assertions outside the supplied targets are outside this batch, not demonstrated omissions from the original extraction. Source support does not establish historical truth.',
            'PROPOSAL SCHEMA (representation contract, not evidence):\n'+json.dumps(Assertion.model_json_schema(),separators=(',',':'))+
            '\n\nSOURCE PACKET (metadata is not evidence):\n'+packet_prompt(task['packet']).split('\n',1)[1]+'\n\nPROPOSALS TO REVIEW (not source evidence):\n'+json.dumps(task['targets'],ensure_ascii=False),SemanticReview)


def inspect_review(text,task):
    expected={t['assertion_id']:t for t in task['targets']};seen=set();out={'judgments':[],'invalid_judgments':[],'errors':[],'missing':[],'scope':'Editorial model proposals only; no scientific endorsement or import.'}
    segments=[task['packet']['primary'],*task['packet']['context']]
    try:
        raw=strict_json(text)
        if not isinstance(raw,dict) or not isinstance(raw.get('judgments'),list):
            raise ValueError('review requires a judgments list')
    except (TypeError,ValueError) as exc:
        out.update(errors=[str(exc)],aligned=False,missing=sorted(expected));return out
    if set(raw)!={'judgments','omissions','context_gaps'}:out['errors'].append('missing or extra review envelope fields')
    if len(raw['judgments'])>12:out['errors'].append('review cap exceeded')
    for field in ('omissions','context_gaps'):
        try:out[field]=TypeAdapter(list[str]).validate_python(raw.get(field),strict=True)
        except (TypeError,ValueError) as exc:out['errors'].append(field+': '+str(exc))
    for index,item in enumerate(raw['judgments']):
        try:j=Judgment.model_validate(item,strict=True)
        except (TypeError,ValueError) as exc:
            out['invalid_judgments'].append({'index':index,'raw':item,'raw_digest':digest(item),'error':str(exc)})
            out['errors'].append('invalid judgment '+str(index));continue
        row=j.model_dump();row['quote_spans']=quote_spans(j.evidence_quote,segments)
        valid=j.assertion_id in expected and j.assertion_id not in seen
        if not valid:out['errors'].append('unknown or duplicate assertion ID: '+j.assertion_id)
        seen.add(j.assertion_id);row['target_known_once']=valid
        # A model cannot waive failed mechanical provenance. This is only an
        # eligibility diagnostic, never an import decision or historical truth.
        target=expected.get(j.assertion_id,{})
        row['eligible_for_editorial_consideration']=valid and bool(row['quote_spans']) and target.get('mechanically_ready',False) and target.get('primary_evidence',False) and j.disposition=='supported'
        out['judgments'].append(row)
    out['missing']=sorted(set(expected)-seen);out['aligned']=not out['errors'] and not out['missing']
    if not out['aligned']:
        for row in out['judgments']:row['eligible_for_editorial_consideration']=False
    return out
