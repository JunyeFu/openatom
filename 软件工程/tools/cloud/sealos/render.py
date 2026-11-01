"""Render only non-secret Sealos platform values; never contacts Kubernetes."""
import argparse
import json
from pathlib import Path
import re


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for option in ['namespace', 'image-digest-ref', 'public-host', 'https-tls-secret', 'ingress-class']:
        parser.add_argument('--' + option, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    values = {key.upper(): value for key, value in vars(args).items() if key != 'output'}
    values['PUBLIC_HOST_PREFIX'] = args.public_host.split('.', 1)[0]
    if not re.fullmatch(r'[A-Za-z0-9._:/-]+@sha256:[0-9a-f]{64}', args.image_digest_ref):
        parser.error('--image-digest-ref must be the registry repository@sha256 digest AFTER push')
    # Public platform values must not inject template syntax or additional resources.
    for key, value in values.items():
        if key != 'IMAGE_DIGEST_REF' and not re.fullmatch(r'[a-z0-9][a-z0-9.-]*', value):
            parser.error(f'invalid non-secret platform value: {key}')
    args.output.mkdir(parents=True, exist_ok=True)
    for name in ['migration', 'application']:
        text = (Path(__file__).parent / f'{name}.template.json').read_text(encoding='utf-8')
        for key, value in values.items():
            text = text.replace(f'__{key}__', value)
        if re.search(r'__[A-Z_]+__', text):
            raise ValueError('unresolved template placeholder')
        json.loads(text)
        (args.output / f'{name}.json').write_text(text, encoding='utf-8')
    print(str(args.output.resolve()))


if __name__ == '__main__':
    main()
