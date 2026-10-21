//! Collision world placement from the disc against the game's own objects in RAM (`context/ram/s05.bin`, save-state
//! slot 5 on court 10; not in git, skipped when absent along with the disc image). The plant records that become
//! props (trees, props, structures with a model and a non-zero scale) are instanced with `world::place` /
//! `world::instance` and walked in the game's list order; each must match its RAM object bit for bit: model,
//! net-post flag, scale, world matrix, root node matrices, bounding sphere centre and — for collision models — the
//! world → model matrix. The 20 m grid built from the collision props must match the game's bounds and cell lists.

use hst_data::fixture;
use hst_data::xb::Archive;
use hst_data::{layout, mdl, mtl};
use hst_sim::world::{self, M4};

const LIST: u32 = 0x423928;
const GRID: u32 = 0x4238a8;
const GRID_MINMAX: u32 = 0x4238c0;
const GRID_BOUNDS: u32 = 0x4238e0;
const GRID_DIMS: u32 = 0x423910;

struct Ram(Vec<u8>);
impl Ram {
    fn bytes(&self, a: u32, n: usize) -> &[u8] {
        let a = (a & 0x1ff_ffff) as usize;
        &self.0[a..a + n]
    }
    fn u(&self, a: u32) -> u32 {
        u32::from_le_bytes(self.bytes(a, 4).try_into().unwrap())
    }
    fn i(&self, a: u32) -> i32 {
        self.u(a) as i32
    }
    fn f(&self, a: u32) -> f32 {
        f32::from_bits(self.u(a))
    }
    fn m4(&self, a: u32) -> M4 {
        std::array::from_fn(|r| std::array::from_fn(|k| self.f(a + 16 * r as u32 + 4 * k as u32)))
    }
}

fn bits<const N: usize>(v: &[[f32; 4]; N]) -> Vec<u32> {
    v.iter().flatten().map(|x| x.to_bits()).collect()
}

#[test]
fn court_10_props_and_grid_match_ram() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let (Ok(ram), Ok(mut iso)) = (fixture::read(format!("{root}/context/ram/s05.bin")), fixture::iso())
    else {
        fixture::skip(format_args!("RAM dump or disc missing, skipped"));
        return;
    };
    let r = Ram(ram);
    let mut files = std::collections::HashMap::new();
    let (mut list, mut plants) = (Vec::new(), Vec::new());
    for arc in ["CMN.XB", "GRD01.XB", "HOL01.XB"] {
        let data = iso.read(&format!("COURT/10/{arc}")).unwrap();
        let a = Archive::parse(&data).unwrap();
        let read = |n: &str| a.entries.iter().find(|e| e.name.eq_ignore_ascii_case(n)).map(|e| a.read(e).unwrap());
        for e in &a.entries {
            let name = e.name.to_ascii_lowercase();
            if name.ends_with("entry_c10.txt") {
                list = layout::entries(&String::from_utf8_lossy(&a.read(e).unwrap()));
            } else if name.ends_with("plant_c10_h01_0.dat") {
                plants = layout::plants(&a.read(e).unwrap()).unwrap();
            } else if let Some(stem) = name.strip_suffix(".mdl") {
                let base = &e.name[..e.name.len() - 4];
                let m = mdl::parse(&a.read(e).unwrap()).unwrap();
                let mt = mtl::parse(&read(&format!("{base}.MTL")).unwrap(), read(&format!("{base}.MTI")).as_deref()).unwrap();
                let size = a.read(e).unwrap().len();
                files.insert(stem.rsplit(['/', '\\']).next().unwrap().to_string(), (m, mt, size));
            }
        }
    }

    // props in creation order: the game walks categories in ascending order, records in file order
    let mut created = Vec::new();
    for cat in [15, 17, 18, 19, 20] {
        for p in plants.iter().filter(|p| p.category == cat && p.scale != 0.0) {
            let Some(e) = layout::resolve(&list, p, 0) else { continue };
            let Some((model, mt, size)) = files.get(&e.stem) else { panic!("{}: no model on disc ({:?})", e.stem, files.keys().collect::<Vec<_>>()) };
            let (m, at_origin) = world::place(p.category, p.pos, p.yaw, p.code);
            created.push((e.stem.clone(), world::instance(m, p.scale, &model.node_local[0], model.center, model.radius), p.scale, at_origin, model.collides(mt), *size));
        }
    }

    let mut node = r.u(LIST);
    let mut ids = Vec::new(); // RAM list node of each prop, in list order
    let mut colliding = Vec::new();
    for k in world::list_order(created.len()) {
        let (stem, prop, scale, at_origin, collides, size) = &created[k];
        assert_ne!(node, 0, "RAM list shorter than {} props", created.len());
        let obj = r.u(r.u(r.u(node)));
        let inst = r.u(obj + 0xc);
        let data = r.u(inst + 0x54);
        let n0 = r.u(inst + 0x64);
        let at = format!("prop {} ({stem})", ids.len());
        assert_eq!(r.u(data + 4) as usize, *size, "{at}: model size");
        assert_eq!(r.bytes(node + 7, 1)[0] != 0, *at_origin, "{at}: net-post flag");
        assert_eq!(r.f(obj + 0x120).to_bits(), scale.to_bits(), "{at}: scale");
        assert_eq!(bits(&r.m4(obj + 0xe0)), bits(&prop.world), "{at}: world matrix");
        assert_eq!(bits(&r.m4(n0)), bits(&prop.node), "{at}: node matrix");
        assert_eq!(bits(&r.m4(n0 + 0x40)), bits(&prop.node_unscaled), "{at}: unscaled node matrix");
        assert_eq!(bits(&[std::array::from_fn(|k| r.f(n0 + 0xc0 + 4 * k as u32))]), bits(&[prop.center]), "{at}: sphere centre");
        assert_eq!(r.bytes(data + 0x48, 1)[0] != 0, *collides, "{at}: collision flag");
        if *collides {
            assert_eq!(bits(&r.m4(inst + 0x10)), bits(&prop.to_model), "{at}: world → model matrix");
            colliding.push((ids.len(), prop.center, prop.radius));
        }
        ids.push(node);
        node = r.u(node + 0x10);
    }
    assert_eq!(node, 0, "RAM list longer than {} props", created.len());

    let g = world::grid(&colliding);
    let minmax: Vec<u32> = (0..3).map(|k| r.u(GRID_MINMAX + 4 * k)).chain((0..3).map(|k| r.u(GRID_MINMAX + 0x10 + 4 * k))).collect();
    assert_eq!(minmax, g.min.iter().chain(&g.max).map(|v| v.to_bits()).collect::<Vec<_>>(), "grid box");
    let b: Vec<i32> = (0..6).map(|k| r.i(GRID_BOUNDS + 8 * k)).collect();
    assert_eq!(b, [g.lo[0], g.hi[0], g.lo[1], g.hi[1], g.lo[2], g.hi[2]], "grid cell bounds");
    assert_eq!([r.i(GRID_DIMS), r.i(GRID_DIMS + 8), r.i(GRID_DIMS + 16)], g.dims(), "grid dims");
    let mut listed = 0;
    for (c, want) in g.cells.iter().enumerate() {
        let cell = r.u(GRID) + 8 * c as u32;
        let got: Vec<usize> = (0..r.i(cell).max(0) as u32).map(|k| ids.iter().position(|&n| n == r.u(r.u(cell + 4) + 4 * k)).unwrap()).collect();
        assert_eq!(&got, want, "cell {c}");
        listed += got.len();
    }
    eprintln!("{} props, {} collide, {} cells, {listed} entries", ids.len(), colliding.len(), g.cells.len());
}

/// Court 5's colliding props have child nodes (the ball-boy cart: wheels and balls under the body, the balls two
/// levels down), unlike court 10's. Against `context/p125/court05.bin` (a doubles match on court 5 at P1's first serve,
/// `context/p125/p125_court.py`, journal 2026-10-21-p125-prop-nodes): every node of every colliding prop has the
/// matrix `court::world` gives it — local · parent from the disc — except the cart's two wheels, which its own motion
/// (`ballboy01.ani`) turns: their axles sit where the walk puts them, their rotation is the motion's (not ported yet).
#[test]
fn court_5_prop_nodes_match_ram() {
    let (Ok(ram), Ok(mut iso)) = (fixture::read("context/p125/court05.bin"), fixture::iso()) else {
        fixture::skip(format_args!("court 5 RAM dump or disc missing, skipped"));
        return;
    };
    let r = Ram(ram);
    assert_eq!(r.u(0x422f90), 5, "the dump isn't on court 5");
    let w = hst_sim::court::world(&mut iso, 5).unwrap();
    // the cart's model and the nodes its motion keys (more than one rotation or position key)
    let hol = iso.read("COURT/05/HOL01.XB").unwrap();
    let a = Archive::parse(&hol).unwrap();
    let file = |end: &str| a.read(a.entries.iter().find(|e| e.name.to_ascii_lowercase().ends_with(end)).unwrap()).unwrap();
    let cart = mdl::parse(&file("ballboy01.mdl")).unwrap();
    let motion = hst_data::ani::parse(&file("ballboy01.ani")).unwrap();
    let moved: Vec<usize> = motion.tracks.iter().filter(|t| t.rotation.len() > 1 || t.position.len() > 1)
        .map(|t| cart.node_names.iter().position(|n| *n == t.name).unwrap()).collect();
    assert_eq!(moved.len(), 2, "the cart's two wheels turn");

    // the RAM list's colliding props, in list order (the order `court::world` keeps them in)
    let mut node = r.u(LIST);
    let mut ram = Vec::new();
    while node != 0 {
        let inst = r.u(r.u(r.u(r.u(node))) + 0xc);
        if r.bytes(r.u(inst + 0x54) + 0x48, 1)[0] != 0 {
            ram.push(r.u(inst + 0x64));
        }
        node = r.u(node + 0x10);
    }
    assert_eq!(ram.len(), w.props.len(), "colliding props");
    let (mut children, mut carts) = (0, 0);
    for (i, (p, &n0)) in w.props.iter().zip(&ram).enumerate() {
        let cart_like = p.nodes.len() == cart.node_count && w.models[p.model].node_from_model == cart.node_inverse;
        carts += cart_like as usize;
        for (k, want) in p.nodes.iter().enumerate() {
            let at = n0 + 0x120 * k as u32;
            if cart_like && moved.contains(&k) {
                // the axle stays where the walk puts it; the wheel's turn is the motion's
                let got = r.m4(at);
                assert_eq!(bits(&[got[3]]), bits(&[want[3]]), "prop {i} node {k}: axle");
                assert_ne!(bits(&got), bits(want), "prop {i} node {k}: the wheel should be turned");
                continue;
            }
            assert_eq!(bits(&r.m4(at)), bits(want), "prop {i} node {k}");
            children += (k > 0) as usize;
        }
    }
    assert!(carts == 4 && children >= 16, "{carts} carts, {children} child nodes checked");
    eprintln!("{} props, {carts} carts, {children} still child nodes bit-exact", w.props.len());
}

/// The colliding props' node arrays in list order (each node 0x120 bytes on: its world matrix, then at +0x80 its
/// local).
fn prop_nodes(r: &Ram) -> Vec<u32> {
    let mut node = r.u(LIST);
    let mut ram = Vec::new();
    while node != 0 {
        let inst = r.u(r.u(r.u(r.u(node))) + 0xc);
        if r.bytes(r.u(inst + 0x54) + 0x48, 1)[0] != 0 {
            ram.push(r.u(inst + 0x64));
        }
        node = r.u(node + 0x10);
    }
    ram
}

/// P133: court 5's carts play `ballboy01.ani` (`court::Motions`). In P125's dump the game has drawn the court 517
/// times since the build: the first pass steps nothing, the 516 others leave each cart's clock at 36 (sampled 35),
/// the locals at the sample at 35 and the world matrices composed from the sample before (34: the dump is taken
/// between a draw pass and the next update, which recomposes before its sweep). The port's 517 passes give the same
/// clock and locals bit for bit, and its world nodes are the 35 pose the next sweep sees.
#[test]
fn court_5_cart_clocks_match_ram() {
    let (Ok(ram), Ok(mut iso)) = (fixture::read("context/p125/court05.bin"), fixture::iso()) else {
        fixture::skip(format_args!("court 5 RAM dump or disc missing, skipped"));
        return;
    };
    let r = Ram(ram);
    assert_eq!(r.u(0x422f90), 5, "the dump isn't on court 5");
    let (mut w, mut mo) = hst_sim::court::moving(&mut iso, 5).unwrap();
    assert_eq!(mo.list.iter().map(|m| m.prop).collect::<Vec<_>>(), [3, 4, 5, 6], "the four carts move");
    for _ in 0..517 {
        mo.pass(&mut w, true, |_| false);
    }
    let ram = prop_nodes(&r);
    for m in &mo.list {
        let n0 = ram[m.prop];
        // the cart's clock: sampled, then time, just before its node array
        assert_eq!((r.f(n0 - 0xb8), r.f(n0 - 0xb4)), (m.clock.sampled, m.clock.time), "prop {} clock", m.prop);
        assert_eq!((m.clock.sampled, m.clock.time), (35.0, 36.0));
        for (k, local) in m.nodes.local.iter().enumerate() {
            assert_eq!(bits(&r.m4(n0 + 0x120 * k as u32 + 0x80)), bits(local), "prop {} node {k} local", m.prop);
        }
        assert_eq!(w.props[m.prop].nodes, m.world_nodes(), "the sweep sees the last sample");
        let mut before = m.clone();
        before.sample(34.0);
        for (k, want) in before.world_nodes().iter().enumerate() {
            assert_eq!(bits(&r.m4(n0 + 0x120 * k as u32)), bits(want), "prop {} node {k} world (sample 34)", m.prop);
        }
    }
}

/// P133: the carts' wheels over 7200 frames of a doubles match on court 5 (`research/journal/2026-10-21-p133-prop-motions/scripts/p133_wheels.py`:
/// `context/p133/run2.bin`, P1's first serve on, two pauses). Each record: vsync, match phase, the four carts'
/// clocks (sampled, time) and their two wheels' world matrices, read just after a frame's draw pass. Every record's
/// wheels are the port's pose at the sampled time or the one before (composed at the pass's start, or recomposed
/// since), its time one frame ahead (or, read inside the step, not yet moved on); the clock steps once per vsync, holding only through the two pauses (a read
/// landing on either side of a pass shows as a 0 next to a 2).
#[test]
fn court_5_cart_wheels_follow_their_clock() {
    let (Ok(rec), Ok(mut iso)) = (fixture::read("context/p133/run2.bin"), fixture::iso()) else {
        fixture::skip(format_args!("court 5 wheel recording or disc missing, skipped"));
        return;
    };
    const SIZE: usize = 8 + 4 * 8 + 8 * 0x40;
    assert_eq!(rec.len() % SIZE, 0);
    let (_, mo) = hst_sim::court::moving(&mut iso, 5).unwrap();
    // each cart's wheel world matrices at every sample time
    let poses: Vec<Vec<[Vec<u32>; 2]>> = mo
        .list
        .iter()
        .map(|m| {
            let mut m = m.clone();
            (0..m.length() as i32)
                .map(|t| {
                    m.sample(t as f32);
                    let n = m.world_nodes();
                    [bits(&n[1]), bits(&n[2])]
                })
                .collect()
        })
        .collect();
    let len = mo.list[0].length();
    assert_eq!(len, 40.0);
    let f = |b: &[u8], o: usize| f32::from_le_bytes(b[o..o + 4].try_into().unwrap());
    let (mut prev, mut steps, mut holds, mut lagging, mut mid) = (None::<(u32, [f32; 4])>, vec![], 0u32, 0, 0);
    let (mut elapsed, mut run) = (0, 0);
    for b in rec.chunks(SIZE) {
        let v = u32::from_le_bytes(b[..4].try_into().unwrap());
        let mut sampled = [0.0; 4];
        for c in 0..4 {
            let (s, t) = (f(b, 8 + 8 * c), f(b, 12 + 8 * c));
            // a read inside the step itself sees the sample taken and the time not yet moved on
            assert!(t == s + 1.0 || t == s, "vsync {v} cart {c}: time {t} one frame ahead of {s}");
            mid += (t == s) as usize;
            sampled[c] = s;
            let wheel = |k: usize| -> Vec<u32> {
                let o = 40 + 0x40 * (4 * k + c);
                (0..16).map(|i| u32::from_le_bytes(b[o + 4 * i..o + 4 * i + 4].try_into().unwrap())).collect()
            };
            let got = [wheel(0), wheel(1)];
            let at = |t: f32| &poses[c][t as usize];
            if got == *at(s) {
                continue;
            }
            assert_eq!(got, *at((s + len - 1.0) % len), "vsync {v} cart {c}: wheels at the sample {s} or the one before");
            lagging += 1;
        }
        if let Some((pv, ps)) = prev {
            // the recorder drops a frame it couldn't read cleanly
            assert!((1..=3).contains(&(v - pv)), "vsync {pv} → {v}");
            elapsed += v - pv;
            for c in 0..4 {
                let d = (sampled[c] - ps[c] + len) % len;
                assert!(d <= (v - pv + 1) as f32, "vsync {v} cart {c}: {d} steps");
                steps.push(d);
            }
            run = if sampled == ps { run + 1 } else { 0 };
            // a pause: the clocks held for longer than a read's jitter (a lone 0 beside a 2)
            holds += match run {
                10 => 10,
                11.. => 1,
                _ => 0,
            };
        }
        prev = Some((v, sampled));
    }
    let total: f32 = steps.iter().sum();
    // per cart: one step per vsync but the paused frames
    assert_eq!(total, 4.0 * (elapsed - holds) as f32, "{holds} held frames");
    assert!((150..250).contains(&holds), "the two pauses hold the clocks ({holds} frames)");
    assert!(mid < 20, "{mid} reads inside a step");
    eprintln!("{} records over {elapsed} vsyncs, {holds} held, {lagging} wheel reads a sample behind, {mid} inside a step", rec.len() / SIZE);
}

/// P133: what holds a cart's clock: the first draw pass after the build, a pass that doesn't step (paused, slow
/// motion between steps) and a pass that hides the cart; until its first step a cart keeps its disc pose.
#[test]
fn cart_clocks_hold_on_the_first_pass_paused_and_hidden() {
    let Ok(mut iso) = fixture::iso() else {
        fixture::skip(format_args!("disc missing, skipped"));
        return;
    };
    let (mut w, mut mo) = hst_sim::court::moving(&mut iso, 5).unwrap();
    let disc = w.clone();
    let times = |mo: &hst_sim::court::Motions| mo.list.iter().map(|m| (m.clock.time, m.clock.sampled)).collect::<Vec<_>>();
    mo.pass(&mut w, true, |_| false);
    assert_eq!(times(&mo), [(0.0, 0.0); 4], "the first pass steps nothing");
    mo.pass(&mut w, false, |_| false);
    assert_eq!(times(&mo), [(0.0, 0.0); 4], "paused");
    assert!(w.props.iter().zip(&disc.props).all(|(a, b)| a.nodes == b.nodes), "the disc pose until the first step");
    mo.pass(&mut w, true, |p| p == 4);
    assert_eq!(times(&mo), [(1.0, 0.0), (0.0, 0.0), (1.0, 0.0), (1.0, 0.0)], "a hidden cart holds");
    for _ in 0..40 {
        mo.pass(&mut w, true, |_| false);
    }
    assert_eq!(times(&mo), [(1.0, 0.0), (40.0, 39.0), (1.0, 0.0), (1.0, 0.0)], "looping: time 40 wraps on the next step");
    let still: Vec<_> = (0..w.props.len()).filter(|p| !(3..7).contains(p)).collect();
    assert!(still.iter().all(|&p| w.props[p].nodes == disc.props[p].nodes), "only the carts move");
}
