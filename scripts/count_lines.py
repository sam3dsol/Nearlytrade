# Code lines in scope: non-blank lines that are not comments, outside `#[cfg(test)] mod` blocks.
# usage: python3 scripts/count_lines.py contracts/*/src/lib.rs
import sys
for path in sys.argv[1:]:
    s = open(path).read().split('\n'); n = len(s); test = [False] * n; i = 0
    while i < n:
        if s[i].startswith('#[cfg(test)]') and i + 1 < n and s[i + 1].startswith('mod '):
            depth = 0; j = i + 1; test[i] = True
            while j < n:
                depth += s[j].count('{') - s[j].count('}'); test[j] = True; j += 1
                if depth == 0: break
            i = j
        else: i += 1
    code = sum(1 for k, l in enumerate(s) if not test[k] and l.strip() and not l.strip().startswith('//'))
    print(f"{path}: {code} code lines, {sum(test)} test lines")
