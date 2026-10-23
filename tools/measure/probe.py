"""Counters for a settlement measurement, patched into a copy of the tree.

    python3 tools/measure/probe.py TREE

Adds eprintln! lines the run script greps for: DEATH (each death and its
cause), MOVECAMP (each camp move), and one line per successful action of
interest (KEEPX food keeping, SHELX shelter, CAREX care and water, GAPX
salt, bowls, boiling, FISHX fishing, LEATHX hides and garments, HERDX kept beasts, POTX pots, SALTX salt, SANX waste and stores), plus GAPX windburn per turn of it.
Each patch is skipped with a note where the tree does not have its anchor,
so the same script serves older builds.
"""
import os
import sys

root = sys.argv[1]


def edit(rel, old, new):
    p = os.path.join(root, rel)
    t = open(p).read()
    if old not in t:
        print(f"skip {rel}: anchor not found")
        return
    assert t.count(old) == 1, (rel, old[:60])
    open(p, "w").write(t.replace(old, new))
    print(f"ok {rel}")


edit(
    "src/agents/population.rs",
    "                let killed_by = agent.emotions.recent_attacker(self.current_turn);\n",
    "                let killed_by = agent.emotions.recent_attacker(self.current_turn);\n"
    "                eprintln!(\"DEATH D{:04} {} age={} cause={}\", self.current_turn / crate::environment::seasons::TICKS_PER_DAY, agent.id, agent.state.age, cause_str);\n",
)

edit(
    "src/analytics/wanting/camp.rs",
    "        let mut going: Vec<uuid::Uuid> = vec![self.population.agents[decider].id];\n",
    "        eprintln!(\"MOVECAMP D{:04} {}\", self.current_turn / crate::environment::seasons::TICKS_PER_DAY, mates.len() + 1);\n"
    "        let mut going: Vec<uuid::Uuid> = vec![self.population.agents[decider].id];\n",
)

# One line per successful action of interest. Variants an older tree lacks
# are matched by name through Debug, so this compiles against any of them.
edit(
    "src/analytics/turn/each_one.rs",
    "            self.population.agents[agent_index].by_what_way = None;\n"
    "            let action_result = self.execute_action(&action, agent_index);\n",
    "            self.population.agents[agent_index].by_what_way = None;\n"
    "            let action_result = self.execute_action(&action, agent_index);\n"
    "            if action_result.success {\n"
    "                let named = format!(\"{:?}\", action);\n"
    "                let head = named.split(|c: char| !c.is_alphanumeric()).next().unwrap_or(\"\").to_string();\n"
    "                let line = match head.as_str() {\n"
    "                    \"Dry\" | \"Smoke\" | \"Salt\" | \"Brine\" | \"Cache\" | \"PutUpRack\" | \"HangUp\" | \"StoneBoil\" | \"PitOven\"\n"
    "                    | \"Cook\" | \"Excavate\" | \"Cover\" => format!(\"KEEPX {}\", head.to_lowercase()),\n"
    "                    \"Boil\" => \"GAPX boil\".to_string(),\n"
    "                    \"Fish\" | \"ForageShallows\" | \"SetFishTrap\" | \"CheckFishTraps\" => format!(\"FISHX {}\", head.to_lowercase()),\n"
    "                    \"LayBedding\" => if action_result.message.as_deref().is_some_and(|m| m.contains(\"hide\")) { \"SHELX bed hide\".to_string() } else { \"SHELX bed\".to_string() },\n"
    "                    \"Build\" => {\n"
    "                        let m = action_result.message.clone().unwrap_or_default();\n"
    "                        let how = if m.starts_with(\"Finished\") { \"finished\" } else if m.starts_with(\"Started\") { \"started\" } else { \"worked\" };\n"
    "                        let what = named.split('\"').nth(1).unwrap_or(\"?\").to_string();\n"
    "                        format!(\"SHELX build {what} {how}\")\n"
    "                    }\n"
    "                    \"PressOnWound\" | \"Bandage\" => format!(\"CAREX {} {}\", head.to_lowercase(), if named.contains(\"Some\") { \"other\" } else { \"self\" }),\n"
    "                    \"Gather\" if named.contains(\"\\\"salt\\\"\") => \"GAPX salt\".to_string(),\n"
    "                    \"Work\" if named.contains(\"\\\"carve\\\"\") && named.contains(\"\\\"wood\\\"\") => \"GAPX bowl\".to_string(),\n"
    "                    \"Work\" if named.contains(\"\\\"hides\\\"\") && named.contains(\"\\\"weave\\\"\") => \"CAREX waterskin\".to_string(),\n"
    "                    \"Work\" if named.contains(\"\\\"scrape\\\"\") => \"LEATHX scrape\".to_string(),\n"
    "                    \"Work\" if named.contains(\"\\\"rawhide\\\"\") && named.contains(\"\\\"fire\\\"\") => \"LEATHX tan\".to_string(),\n"
    "                    \"MakeClothing\" => format!(\"LEATHX wear {}\", named.split('\"').nth(1).unwrap_or(\"?\")),\n"
    "                    \"Hunt\" if action_result.message.as_deref().is_some_and(|m| m.contains(\"young one\")) => \"GAPX hunt\\nHERDX orphan\".to_string(),\n"
    "                    \"Work\" if named.contains(\"\\\"mold\\\"\") && named.contains(\"\\\"clay\\\"\") => \"POTX shaped\".to_string(),\n"
    "                    \"Work\" if named.contains(\"\\\"fire\\\"\") && named.contains(\"\\\"claypot\\\"\") => \"POTX fired\".to_string(),\n"
    "                    \"Gather\" if named.contains(\"\\\"clay\\\"\") => \"POTX clay\".to_string(),\n"
    "                    \"Hunt\" => \"GAPX hunt\".to_string(),\n"
    "                    \"StartMidden\" => \"SANX midden\".to_string(),\n"
    "                    \"DigLatrine\" => \"SANX latrine\".to_string(),\n"
    "                    \"Tame\" => { let m = action_result.message.clone().unwrap_or_default(); format!(\"HERDX {}\", if m.starts_with(\"Caught\") { \"caught\" } else if m.contains(\"is tame\") { \"tamed\" } else { \"fed\" }) }\n"
    "                    \"CollectAnimalProduct\" => format!(\"HERDX took {}\", if action_result.message.as_deref().is_some_and(|m| m.contains(\"wool\")) { \"wool\" } else { \"milk\" }),\n"
    "                    \"PutUpFold\" => \"HERDX fold\".to_string(),\n"
    "                    \"MuckOut\" => \"HERDX muckout\".to_string(),\n"
    "                    \"Slaughter\" => \"HERDX slaughter\".to_string(),\n"
    "                    _ => String::new(),\n"
    "                };\n"
    "                if !line.is_empty() { eprintln!(\"{line}\"); }\n"
    "            }\n",
)

edit(
    "src/environment/exposure.rs",
    "            self.active_exposures.push(ExposureType::Windburn);\n",
    "            self.active_exposures.push(ExposureType::Windburn);\n"
    "            eprintln!(\"GAPX windburn\");\n",
)

edit(
    "src/analytics/wanting/wounds.rs",
    "        Action::Gather { resource_type: \"water\".to_string() }\n    }",
    "        eprintln!(\"CAREX fillforwalk\");\n        Action::Gather { resource_type: \"water\".to_string() }\n    }",
)

edit(
    "src/agents/agent.rs",
    "                self.hides_that_rotted += item.quantity;\n",
    "                self.hides_that_rotted += item.quantity;\n"
    "                eprintln!(\"LEATHX rotted {}\", item.quantity);\n",
)

edit(
    "src/analytics/happening/herds.rs",
    "    fn turned_loose(animal: &mut crate::environment::fauna::Animal) {\n",
    "    fn turned_loose(animal: &mut crate::environment::fauna::Animal) {\n"
    "        eprintln!(\"HERDX lost {}\", if animal.is_domesticated { \"tame\" } else { \"raising\" });\n",
)

edit(
    "src/agents/agent.rs",
    "        self.pots_broken += 1;\n",
    "        self.pots_broken += 1;\n"
    "        eprintln!(\"POTX broken\");\n",
)

edit(
    "src/analytics/doing/eating.rs",
    "            let stewed = self.stewing_a_batch(agent_index, food_type, fire_id);\n            if stewed.success {\n",
    "            let stewed = self.stewing_a_batch(agent_index, food_type, fire_id);\n            if stewed.success {\n"
    "                eprintln!(\"POTX stew\");\n",
)

edit(
    "src/agents/agent.rs",
    "            self.salt_in_the_body = (self.salt_in_the_body + Self::WHAT_A_PINCH_PUTS_BACK).min(1.0);\n",
    "            self.salt_in_the_body = (self.salt_in_the_body + Self::WHAT_A_PINCH_PUTS_BACK).min(1.0);\n"
    "            eprintln!(\"SALTX lick\");\n",
)

edit(
    "src/agents/agent.rs",
    "        if self.salt_in_the_body <= 0.0 {\n            self.take_damage(",
    "        if self.salt_in_the_body <= 0.0 {\n            eprintln!(\"SALTX without\");\n            self.take_damage(",
)

edit(
    "src/agents/agent.rs",
    "        let salt = if salted { 0.1 } else { Self::salt_in_a_mouthful_of(item_id) };\n",
    "        let salt = if salted { 0.1 } else { Self::salt_in_a_mouthful_of(item_id) };\n"
    "        if self.craves_salt() { eprintln!(\"SALTX craving_meal\"); }\n",
)

for what in ("salt: gathered near", "salt: set out on a journey", "journey: given up", "journey: home again", "journey: turned for home"):
    counted = what.replace(": ", "_").replace(" ", "_")
    edit(
        "src/analytics/wanting/journey.rs",
        '.entry("' + what + '".to_string())',
        '.entry({ eprintln!("SALTX ' + counted + '"); "' + what + '".to_string() })',
    )

edit(
    "src/analytics/doing/getting.rs",
    "                        agent.taken_ill_with(crate::agents::Agent::OFF_FOUL_WATER, 0.3 + 0.4 * foul, self.current_turn);\n",
    "                        agent.taken_ill_with(crate::agents::Agent::OFF_FOUL_WATER, 0.3 + 0.4 * foul, self.current_turn);\n"
    "                        eprintln!(\"SANX ill_foulwater\");\n",
)

edit(
    "src/analytics/happening/soil.rs",
    "                agent.what_filth_teaches();\n",
    "                agent.what_filth_teaches();\n"
    "                eprintln!(\"SANX ill_foulground\");\n",
)

edit(
    "src/analytics/wanting/camp.rs",
    "                self.propose_moving_camp(agent, (there.x, there.y), None, \"fouled ground\");\n",
    "                self.propose_moving_camp(agent, (there.x, there.y), None, \"fouled ground\");\n"
    "                eprintln!(\"SANX move_fouled\");\n",
)

edit(
    "src/analytics/wanting/camp.rs",
    "                self.propose_moving_camp(agent, (there.0, there.1), None, \"fresh grazing\");\n",
    "                self.propose_moving_camp(agent, (there.0, there.1), None, \"fresh grazing\");\n"
    "                eprintln!(\"HERDX move_grazing\");\n",
)

edit(
    "src/analytics/doing/keeping.rs",
    "                    self.world.stores_robbed.insert(here, (me, already + taking));\n",
    "                    self.world.stores_robbed.insert(here, (me, already + taking));\n"
    "                    eprintln!(\"SANX store_theft\");\n",
)
