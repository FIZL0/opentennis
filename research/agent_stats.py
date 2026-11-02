#!/usr/bin/env python3
"""Per-session stats for runner transcripts (Claude Code or Pi .jsonl): task, wall time, turns, tool calls,
tokens, peak context. Usage: agent_stats.py FILE.jsonl... (prints one TSV row each)."""
import json, re, sys
from datetime import datetime

def ts(s):
    return datetime.fromisoformat(s.replace('Z', '+00:00')) if s else None

def stats(path):
    task, times, turns, tools, out, inp, cread, cwrite, peak, model = None, [], 0, 0, 0, 0, 0, 0, 0, set()
    for line in open(path, encoding='utf-8', errors='replace'):
        try: d = json.loads(line)
        except ValueError: continue
        t = d.get('timestamp')
        if t: times.append(ts(t) if isinstance(t, str) else datetime.fromtimestamp(t / 1000).astimezone())
        m = d.get('message') or {}
        role = m.get('role') or d.get('type')
        content = m.get('content')
        if task is None and role == 'user':
            text = content if isinstance(content, str) else ' '.join(
                c.get('text', '') for c in content or [] if isinstance(c, dict))
            if (g := re.search(r'Do task (\S+) only', text)): task = g.group(1)
        if role != 'assistant': continue
        if isinstance(content, list):
            tools += sum(1 for c in content if isinstance(c, dict) and c.get('type') in ('tool_use', 'toolCall'))
        u = m.get('usage') or {}
        if not u: continue
        turns += 1
        model.add(m.get('model', '?'))
        i, r, w, o = (u.get('input_tokens', u.get('input', 0)), u.get('cache_read_input_tokens', u.get('cacheRead', 0)),
                      u.get('cache_creation_input_tokens', u.get('cacheWrite', 0)), u.get('output_tokens', u.get('output', 0)))
        inp, cread, cwrite, out = inp + i, cread + r, cwrite + w, out + o
        peak = max(peak, i + r + w)
    wall = (max(times) - min(times)).total_seconds() / 60 if times else 0
    return [task or '?', f'{wall:.0f}', turns, tools, inp, cread, cwrite, out, peak, ','.join(sorted(model)), path]

if __name__ == '__main__':
    print('\t'.join('task wall_min turns tools input cache_read cache_write output peak_ctx model file'.split()))
    for p in sys.argv[1:]:
        print('\t'.join(map(str, stats(p))))
