#!/usr/bin/env python3
"""Offline PDF renderer. Called only after export-guidance.cjs validates selection."""
import hashlib
import io
import json
import logging
from pathlib import Path
import re
import sys
import unicodedata
from urllib.parse import unquote, urlsplit

import pypdf
from pypdf import PdfReader, PdfWriter
from pypdf.generic import DictionaryObject, NameObject, TextStringObject
import weasyprint
from weasyprint import HTML, default_url_fetcher


def normalized(text):
    return re.sub(r'\s+', '', unicodedata.normalize('NFKC', text).replace('\u00ad', ''))


def render(root):
    data = json.loads((root / 'input.json').read_text())
    manifest = data['manifest']
    errors = []
    class Capture(logging.Handler):
        def emit(self, record):
            if record.levelno >= logging.WARNING:
                errors.append(record.getMessage())
    logger = logging.getLogger('weasyprint')
    capture = Capture()
    logger.addHandler(capture)
    def fetch(url, *args, **kwargs):
        parsed = urlsplit(url)
        file = Path(unquote(parsed.path)).resolve()
        if parsed.scheme != 'file' or parsed.netloc or file.parent != root / 'assets' or not file.is_file():
            raise ValueError('PDF renderer blocked resource: '+url)
        return default_url_fetcher(url, *args, **kwargs)
    rendered = {}
    try:
        for doc in data['documents']:
            document = HTML(string=doc['html'], base_url=str(root)+'/', url_fetcher=fetch).render()
            if errors:
                raise ValueError('PDF rendering warning: '+'; '.join(errors))
            anchors = {}
            for number, page in enumerate(document.pages):
                # Fail closed on an unsupported layout API or content outside the page.
                # This is exercised when accepting any replacement renderer version.
                for box in page._page_box.descendants():
                    if getattr(box, 'text', '') or type(box).__name__ in ('InlineReplacedBox', 'BlockReplacedBox', 'AbsolutePlaceholder'):
                        if (box.position_x < -0.5 or box.position_y < -0.5 or
                            box.position_x + box.width > page.width + 0.5 or
                            box.position_y + box.height > page.height + 0.5):
                            raise ValueError('PDF content exceeds page bounds in '+doc['filename'])
                for name in page.anchors:
                    # A section split across physical pages repeats its anchor in the layout.
                    anchors.setdefault(name, number)
            reader = PdfReader(io.BytesIO(document.write_pdf()), strict=True)
            extracted = []
            for number, page in enumerate(reader.pages, 1):
                lines = page.extract_text().splitlines()
                if not lines or lines[-1].strip() != str(number):
                    raise ValueError('PDF page footer is missing')
                extracted.append('\n'.join(lines[:-1]))
            actual = normalized(''.join(extracted))
            for block in doc['blocks']:
                if normalized(block) not in actual:
                    raise ValueError('PDF lost a selected content block in '+doc['filename'])
            for expected in doc['texts']:
                # Page footers can interrupt text at page breaks; require ordered coverage.
                position = 0
                for char in normalized(expected):
                    position = actual.find(char, position)
                    if position < 0:
                        raise ValueError('PDF lost selected text in '+doc['filename'])
                    position += 1
            if errors:
                raise ValueError('PDF rendering warning: '+'; '.join(errors))
            rendered[doc['filename']] = (doc, reader, anchors)
        result = root / 'result'
        result.mkdir()
        for filename, (doc, reader, anchors) in rendered.items():
            writer = PdfWriter(clone_from=reader)
            writer.add_metadata({'/Title':doc['title'], '/Subject':f"{manifest['toe_reference']}; release {manifest['tag']}; commit {manifest['commit']}", '/Creator':'Sequent controlled guidance export'})
            if not set(anchors) <= set(reader.named_destinations):
                raise ValueError('PDF renderer omitted a named destination')
            for page in writer.pages:
                for ref in page.get('/Annots', []):
                    annotation = ref.get_object()
                    action = annotation.get('/A')
                    if not action or action.get('/S') != '/URI':
                        continue
                    url = urlsplit(str(action['/URI']))
                    if url.netloc == 'guidance.invalid':
                        target, anchor = url.path.lstrip('/'), unquote(url.fragment)
                        if target not in rendered or anchor not in rendered[target][2]:
                            raise ValueError('Unresolved offline PDF link')
                        annotation[NameObject('/A')] = DictionaryObject({NameObject('/S'):NameObject('/GoToR'), NameObject('/F'):TextStringObject(target), NameObject('/D'):TextStringObject(anchor)})
                    elif url.scheme not in ('https', 'http', 'mailto'):
                        raise ValueError('Unsafe PDF link: '+str(action['/URI']))
            with (result / filename).open('wb') as stream:
                writer.write(stream)
            for handbook in manifest['documents']:
                if handbook['pdf'] == filename:
                    handbook['page_count'] = len(reader.pages)
                    for source in handbook['pages']:
                        source['start_page'] = anchors[source['anchor']] + 1
        manifest['renderer'] = {'weasyprint':weasyprint.__version__, 'pypdf':pypdf.__version__}
        manifest['files'] = {p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(result.glob('*.pdf'))}
        (result / 'manifest.json').write_text(json.dumps(manifest, indent=2)+'\n')
    finally:
        logger.removeHandler(capture)


if __name__ == '__main__':
    render(Path(sys.argv[1]).resolve())
