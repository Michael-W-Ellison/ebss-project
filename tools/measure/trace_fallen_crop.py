"""Trace each fall of field crop to the field's origin (#426).

    python3 tools/measure/trace_fallen_crop.py TREE

Patched into a measurement tree after probe.py. Adds a FART line for every
fall of field crop: its distance band, who worked this season's crop and
where their hearths are now, who broke the field and who last sowed it, with
where their hearths were then and are now. Add ^FART to run.sh's grep to keep
the lines; read them with fallen_crop_sources.py. Does not change behaviour.
"""
import os, sys
root = sys.argv[1]
def edit(rel, old, new):
    p = os.path.join(root, rel); t = open(p).read()
    assert t.count(old) == 1, (rel, old[:70], t.count(old))
    open(p, "w").write(t.replace(old, new)); print("ok", rel)

edit("src/world/mod.rs", "use std::collections::BTreeMap;\nuse rand::Rng;\n", """use std::collections::BTreeMap;
use rand::Rng;
/// breaker, breaker hearth then, break turn, how (0 new ground, 1 ploughed in, 2 a field already), last sower, sower hearth then, sow turn
pub type TraceOrigin = (uuid::Uuid, Option<(i32, i32)>, u32, u8, uuid::Uuid, Option<(i32, i32)>, u32);
pub static TRACE_ORIGIN: std::sync::Mutex<BTreeMap<(i32, i32), TraceOrigin>> = std::sync::Mutex::new(BTreeMap::new());
pub static TRACE_FALL_WHO: std::sync::Mutex<Vec<Vec<uuid::Uuid>>> = std::sync::Mutex::new(Vec::new());
""")
edit("src/world/mod.rs",
"                    self.field_crop_fallen_where.push((resource.position, fell));\n",
"""                    self.field_crop_fallen_where.push((resource.position, fell));
                    TRACE_FALL_WHO.lock().unwrap().push(resource.put_into_this_crop.iter().map(|(w, _)| *w).collect());
""")
edit("src/analytics/doing/ground.rs",
"        self.world.grid.break_ground(&tile_position, now);\n",
"""        {
            let agent = &self.population.agents[agent_index];
            let fresh = !crate::world::Terrain::new(ground).is_cultivated();
            let mut map = crate::world::TRACE_ORIGIN.lock().unwrap();
            let e = map
                .entry((tile_position.x, tile_position.y))
                .or_insert((agent.id, agent.hearth, now, if fresh { 0 } else { 2 }, agent.id, agent.hearth, now));
            e.4 = agent.id;
            e.5 = agent.hearth;
            e.6 = now;
        }
        self.world.grid.break_ground(&tile_position, now);
""")
edit("src/analytics/doing/ground.rs",
"        self.world.grid.break_ground(&where_it_stands, now);\n",
"""        {
            let agent = &self.population.agents[agent_index];
            crate::world::TRACE_ORIGIN.lock().unwrap()
                .entry((where_it_stands.x, where_it_stands.y))
                .or_insert((agent.id, agent.hearth, now, 1, agent.id, agent.hearth, now));
        }
        self.world.grid.break_ground(&where_it_stands, now);
""")
edit("src/analytics/happening/soil.rs",
"""        for (field, amount) in fallen {
            let band = self.how_far_from_home((field.x, field.y));
            self.world.field_crop_fallen_by_distance[band] += u64::from(amount);
        }
""",
"""        let who_list = std::mem::take(&mut *crate::world::TRACE_FALL_WHO.lock().unwrap());
        let tpd = crate::environment::seasons::TICKS_PER_DAY;
        let day = self.world.turn / tpd;
        for (i, (field, amount)) in fallen.into_iter().enumerate() {
            let band = self.how_far_from_home((field.x, field.y));
            self.world.field_crop_fallen_by_distance[band] += u64::from(amount);
            let d = |h: (i32, i32)| (h.0 - field.x).abs().max((h.1 - field.y).abs());
            // where a person's hearth is now from the field: -1 no hearth, -2 dead, -3 gone
            let now_of = |id: uuid::Uuid| match self.population.agents.iter().find(|a| a.id == id) {
                None => -3,
                Some(a) if !a.state.is_alive => -2,
                Some(a) => a.hearth.map(d).unwrap_or(-1),
            };
            let moved_of = |id: uuid::Uuid| self.population.agents.iter().find(|a| a.id == id).map(|a| a.hearth_moved_at / tpd).unwrap_or(0);
            let workers: Vec<i32> = who_list.get(i).map(|w| w.iter().map(|x| now_of(*x)).collect()).unwrap_or_default();
            let workers = workers.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",");
            let origin = crate::world::TRACE_ORIGIN.lock().unwrap().get(&(field.x, field.y)).cloned();
            match origin {
                Some((b, bh, bt, how, s, sh, st)) => eprintln!(
                    "FART D{:04} band {} amt {} workers [{}] how {} breaker_then {} breaker_now {} broke_day {} breaker_moved_day {} sower_then {} sower_now {} sown_day {} sower_moved_day {}",
                    day, band, amount, workers, how, bh.map(d).unwrap_or(-1), now_of(b), bt / tpd, moved_of(b),
                    sh.map(d).unwrap_or(-1), now_of(s), st / tpd, moved_of(s)
                ),
                None => eprintln!("FART D{:04} band {} amt {} workers [{}] how none", day, band, amount, workers),
            }
        }
""")
