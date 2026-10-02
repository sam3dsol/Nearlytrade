# token-raw is token-tax without the tax: prints every code line (no comments, no blank lines) of
# token-raw that is not in token-tax, in order. usage: python3 scripts/raw_vs_tax.py
import difflib
def code(p):
    return [l.strip() for l in open(p).read().split('\n') if l.strip() and not l.strip().startswith('//')]
r, t = code('contracts/token-raw/src/lib.rs'), code('contracts/token-tax/src/lib.rs')
sm = difflib.SequenceMatcher(None, r, t, autojunk=False)
shared = sum(b.size for b in sm.get_matching_blocks())
print(f"token-raw {len(r)} code lines: {shared} identical to token-tax, {len(r) - shared} of its own:")
for op, a1, a2, b1, b2 in sm.get_opcodes():
    if op in ('replace', 'delete'):
        for l in r[a1:a2]: print('  ' + l)
