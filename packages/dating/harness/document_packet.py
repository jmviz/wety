"""Bounded article-body parsing and explicit linked-material inventory for evaluation."""
from html.parser import HTMLParser
import hashlib
from pathlib import Path
import re
from urllib.parse import urljoin


class ArticleParser(HTMLParser):
    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.active=False;self.skip=0;self.parts=[];self.links=[];self.images=[];self.sections=[]
        self.heading=None;self.heading_labels=[]
    def handle_starttag(self,tag,attrs):
        attrs=dict(attrs)
        if tag=='article':self.active=True
        if not self.active:return
        if tag in ('script','style','nav'):self.skip+=1
        if tag in ('h1','h2','h3','h4') and not self.skip:self.heading={'level':int(tag[1]),'parts':[]}
        if tag in ('section','h1','h2','h3','h4','p','tr','li','br'):self.parts.append('\n')
        if tag=='section' and attrs.get('id'):self.sections.append(attrs['id'])
        if tag=='a' and attrs.get('href'):self.links.append({'href':attrs['href']})
        if tag=='img' and attrs.get('src'):self.images.append({'src':attrs['src'],'alt':attrs.get('alt')})
    def handle_endtag(self,tag):
        if tag in ('h1','h2','h3','h4') and self.heading:
            self.heading_labels.append({'level':self.heading['level'],'label':''.join(self.heading['parts'])});self.heading=None
        if tag in ('script','style','nav'):self.skip=max(0,self.skip-1)
        if tag=='article':self.active=False
        if self.active and tag in ('h1','h2','h3','h4','p','tr','li'):self.parts.append('\n')
    def handle_data(self,data):
        if self.active and not self.skip:self.parts.append(data)
        if self.active and not self.skip and self.heading:self.heading['parts'].append(data)


def parse_article(path,url):
    raw=Path(path).read_bytes();parser=ArticleParser();parser.feed(raw.decode('utf-8'))
    text=re.sub(r'[ \t\r\f\v]+',' ',''.join(parser.parts))
    text=re.sub(r'\n\s*\n+','\n',text).strip()
    if len(text)<2000:raise ValueError('article body absent or too short; do not label landing page full text')
    attachments=sorted({urljoin(url,l['href']) for l in parser.links if re.search(r'\.(?:pdf|docx?|xlsx?|csv|zip|nex|xml)(?:$|[?#])',l['href'],re.I)})
    headings=[];cursor=0;unaligned=[]
    for h in parser.heading_labels:
        label=re.sub(r'\s+',' ',h['label']).strip()
        match=re.search(r'(?m)^'+re.escape(label)+r'\s*$',text[cursor:]) if label else None
        if match:
            start=cursor+match.start();end=start+len(label);cursor=end
            headings.append({'level':h['level'],'text':label,'start':start,'end':end})
        else:unaligned.append(h)
    return text,{'raw_sha256':hashlib.sha256(raw).hexdigest(),'text_sha256':hashlib.sha256(text.encode()).hexdigest(),
        'parser':'harness.document_packet.ArticleParser/v2','source_url':url,'sections':parser.sections,
        'headings':headings,'unaligned_headings':unaligned,
        'attachments':[{'url':u,'status':'listed_not_supplied'} for u in attachments],
        'images':[dict(i,status='not_supplied_as_pixels') for i in parser.images],
        'scope':'Complete parsed article element, including embedded supplementary text when present; linked files and figure pixels are separate omissions.'}
