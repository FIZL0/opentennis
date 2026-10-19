# python3 tools/test_ctx.py — checks lookups and that the TTL cache picks up an edit once it expires
import os, sys, tempfile, time
sys.path.insert(0, os.path.dirname(__file__))
import ctx

d = tempfile.mkdtemp()
p = os.path.join(d, 't.md')
open(p, 'w').write('# A\n- [x] **P1 — done.**\n- [ ] **P2 — open.**\n  more P2\n## B\nb1\n')
assert ctx.section(p, 'next') == ['- [ ] **P2 — open.**', '  more P2']
assert ctx.section(p, 'b') == ['## B', 'b1']
assert ctx.index(p) == ['# A', '[x] P1', '[ ] P2', '## B']
ctx.TTL = 0.2
open(p, 'w').write('# A\n## B\nb2\n'); os.utime(p, ns=(0, time.time_ns() + 10**9))
assert ctx.section(p, 'b') == ['## B', 'b1']  # inside the TTL: cached
time.sleep(0.25)
assert ctx.section(p, 'b') == ['## B', 'b2']  # expired: edit picked up
# tick / block / archive
todo, done = os.path.join(d, 'todo.md'), os.path.join(d, 'done.md')
open(todo, 'w').write('# T\n\n### S\n\n- [ ] **A1** one\n  more A1\n- [x] **A2** two\n- [ ] **A3** three\n')
open(done, 'w').write('# Done\n\n### S\n\n- [x] **A0** zero\n')
own = os.path.join(d, 'A1.md'); open(own, 'w').write('- [ ] **A1** one\n')
ctx.tick(todo, done, 'A1', d)
assert open(todo).read() == '# T\n\n### S\n\n- [ ] **A3** three\n', open(todo).read()
assert open(done).read() == '# Done\n\n### S\n\n- [x] **A0** zero\n- [x] **A1** one\n  more A1\n- [x] **A2** two\n'
assert open(own).read() == '- [x] **A1** one\n'
ctx.block(todo, 'A3', 'no tiebreak')
assert ctx.section(todo, 'A3') == ['- [~] **A3** three BLOCKED: no tiebreak']
# a split parent goes once its subtasks are all done, not while one is open/blocked; P170 isn't P17's
open(todo, 'w').write('# T\n\n### S\n\n- [ ] **P17** (split into P17a–b)\n- [x] **P17b** b\n- [ ] **P170** x\n'
                      '- [ ] **Q1** (split into Q1a–b)\n- [~] **Q1a** a\n')
open(done, 'w').write('# Done\n\n### S\n\n- [x] **P17a** a\n- [x] **Q1b** b\n')
ctx._cache.clear()  # files rewritten behind ctx's TTL cache
ctx.archive(todo, done)
assert open(todo).read() == '# T\n\n### S\n\n- [ ] **P170** x\n- [ ] **Q1** (split into Q1a–b)\n- [~] **Q1a** a\n', open(todo).read()
assert '- [x] **P17** (split into P17a–b)' in open(done).read()
# brief: a SPEC.md in the task's journal makes it a port phase; open questions in it make it a spec phase
r = tempfile.mkdtemp(); os.makedirs(os.path.join(r, 'plan')); j = os.path.join(r, 'research/journal/2026-01-01-x9-walk')
os.makedirs(j)
open(os.path.join(r, 'plan/TODO.md'), 'w').write('# T\n\n### S\n\n- [ ] **X9** walk\n')
open(os.path.join(r, 'plan/DONE.md'), 'w').write('# Done\n')
ctx._cache.clear()
assert not any(l.startswith('## Port') or l.startswith('## Spec') for l in ctx.brief(r, 'X9'))
open(os.path.join(j, 'SPEC.md'), 'w').write('# Walk\nspeed 2 m/s\n\n## Open questions\nnone\n')
assert any(l.startswith('## Port phase') for l in ctx.brief(r, 'X9'))
open(os.path.join(j, 'SPEC.md'), 'w').write('# Walk\n\n## Open questions\n- turn rate?\n')
assert any(l.startswith('## Spec phase') for l in ctx.brief(r, 'X9'))
print('ok')
