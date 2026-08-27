# Emergent Behavior Society Simulator (EBSS)

A general-purpose AI platform for simulating societies of autonomous agents that learn and adapt through behavioral evolution.

## Overview

EBSS provides a modular framework where agents develop complex behaviors through:
- **Weighted Behavior Trees**: Learned decision-making patterns that evolve with experience
- **Drive-Based Motivation**: 15 core drives (hunger, thirst, safety, curiosity, social, etc.) creating dynamic priorities
- **Genetic Inheritance**: Offspring inherit successful behavioral patterns from parents
- **Memory Systems**: Agents remember locations, storage contents, and other agents
- **Observational Learning**: Young agents learn by following experienced agents
- **Modular Environments**: Plugin architecture for different world rules and game mechanics

Unlike game-specific implementations, EBSS is environment-agnostic, allowing researchers and developers to plug in different rule systems (Minecraft-style survival, Dwarf Fortress-inspired societies, medieval simulations, or entirely novel environments) while maintaining the same core AI architecture.

## Project Status

**Current state**: all four planned phases are implemented. A default
simulation runs a society that feeds itself, waters itself, shelters from the
weather, and reproduces over tens of thousands of ticks. Roughly 135,000 lines
across 252 source files, with 1,878 library tests.

Every build configuration compiles: default, `--features gui`,
`--features bevy_gui` and `--workspace`, with 1,794 tests across the workspace.
The work left is connecting rather than building — several analytics
components are libraries with no caller, and agents cannot yet hear anything.
See
[PROJECT_STATUS.txt](PROJECT_STATUS.txt) for measured detail and
[ISSUES_FOUND.md](ISSUES_FOUND.md) for the current defect list. The
[Software Design Document](EBSS_Software_Design_Document.docx) holds the
original specifications.

## Key Features

- ✅ Behavior Tree Learning: Agents build and evolve decision trees through experience
- ✅ Drive Architecture: 15 core drives create emergent behavior patterns
- ✅ Survival: hunger, thirst, nutrition, body temperature, exposure and shelter
- ✅ Genetic Inheritance: offspring inherit traits and behavior from parents
- ✅ Memory Systems: spatial and episodic memory with decay
- ✅ Social Learning: observation, imitation, gossip and shared knowledge
- ✅ Environment Abstraction: plugin interface, crafting, technology progression
- 🚧 Analytics: emergence detection, metrics and replay exist but the
  simulation loop does not drive them — examples do
- ✅ Fire and cooking: agents gather wood, light campfires and cook at them.
  Only meat, fish and grain are improved by a fire; anything else put over one
  is ruined, and so is anything cooked twice. Burning a batch gets rarer with
  practice
- ✅ Perception: sight is how agents find things — terrain, resources and
  buildings within 25 tiles, refreshed every tick, and the Blind trait takes
  it away. Smell is scaled to what a thing actually gives off: a berry on the
  bush carries about two tiles, water three, flesh six, food that has turned
  nine to twenty, and cooking the whole range
- ✅ Soil and flora: every tile carries nutrients and decaying matter, plants
  grow on whichever of water, light and nutrient they have least of, foliage
  sheds leaf fall that becomes soil, and what rots depends on how dense it was
  and how wet the ground is
- ✅ Farming: agents break open grass into fields and sow them. A field gets at
  more of what the soil holds and carries a heavier crop — it does not grow
  anything faster than that plant's kind can grow
- 🚧 A people without a field goes where the ground already carries something,
  and one that has worked farming out stays. The decision is built and tested —
  stripped ground, somewhere three times better a fortnight off, no crop
  standing here — and over sixteen worlds it changed nothing measurable,
  because there is no camp in this model for it to be a departure from. See
  ISSUES_FOUND.md #13
- ✅ A threat is a threat to what an agent still has to do, and a pack is a
  pack. Two things were wrong with the appraisal. It read danger off the
  animal's own statistics and nothing else, so it was a question about teeth
  rather than about what the teeth would end — what a wolf takes is not a
  man's health, it is every meal, every drink and every night's sleep he had
  left, and that is what `Agent::what_i_stand_to_lose` now measures on the
  drives. And it took the single worst thing in sight and threw the rest away,
  so a man hemmed in by four wolves faced whichever one happened to be
  nearest. Several of a thing now add up, with each behind the first worth
  rather less than the one in front of it: four wolves come to about two and a
  half, which is much worse than one and not four times worse. A man who would
  stand his ground against one now runs from four, which is the specification's
  own example and a test
- ✅ Prey is not a threat. What made something frightening was its
  `attack_damage`, which a rabbit has because a rabbit will bite you if you
  pick it up — so once several of a thing began adding up, a herd of twenty
  reindeer came to about a wolf. What menaces somebody who has done nothing is
  a thing that comes after people; what merely defends itself is a question
  for whoever attacks it, and that is a question about temperament rather than
  teeth. Over twenty-four worlds the settlement had been running 465 times
  where it should run 213, and freezing 194 times where it should freeze 27 —
  most of that last being children hemmed in by deer
- ✅ Five answers to a thing that would kill you, where there used to be two.
  Fight if you can win; run if you cannot; turn and fight anyway if there is
  nowhere to run; go anyway if you cannot lift an arm; and when neither is
  possible, freeze. Neither cornered case existed — an agent with nowhere to
  go went back to gathering berries with a wolf at its elbow — and nor did the
  third answer. Freezing fires about 174 times in a settlement's ten thousand
  ticks, and almost all of it is children: a child cannot fight a wolf, and a
  tired child cannot outrun one either. That clause is what made the branch
  reachable at all; with the body and the health tests alone, not one agent in
  eight worlds ever froze
- ✅ Taking is decided on drive demand, not on temperament. The old decision
  was a chance nudged by Honest and Greedy, and it never looked at what was
  being taken: a man robbed somebody without knowing whether the thing was any
  use to him, and it happened once in eight worlds. Now it is what this would
  answer against what it would cost later — the gain is the urgency of
  whichever drive the thing serves, and the cost runs through the bonds,
  because in this model everything a person gets from other people runs
  through the bonds. On an ordinary day the sums come out against it. A
  primary drive past bearing sets the cost aside altogether, because a man who
  will be dead by morning is not weighing his reputation. Theft went from 0.75
  a world to about 21, and it shows downstream exactly as it should: attacks
  roughly quadrupled. A settlement with thieves in it fights more
- ✅ The beasts have an opinion about us. An animal has two drives worth the
  name — eat, and do not be eaten — and until now it had neither about
  people: `AnimalState::Fleeing` and `AnimalState::Attacking` had been in the
  model since the model had animals and nothing had ever set either, so a deer
  stood placidly in a field while somebody walked up to it with a spear.
  Everything now reads the odds against what is coming at it and either goes
  or turns to face it, and temper decides how kindly it reads them. A rabbit
  never stands its ground however the arithmetic comes out. A bear does not
  run from one man — and the same wolf that would take on a man with nothing
  in his hands thinks better of it when he has a spear
- ✅ The hedgerows keep a year. Growth was seasonal from the beginning and
  what was *standing* was not, so a berry bush that had grown all summer still
  had its berries on it in February — and a settlement that can pick fruit in
  the snow has no reason to put anything by, no lean season to be lean in, and
  no use for a store. Spring gives wild leaf and shoot, which is almost no
  energy and a great deal of everything else; summer gives the first roots and
  pods, which is not a harvest; autumn is when everything else comes on at
  once; and winter gives nothing at all. What is on a plant falls off it inside
  a fortnight of its season turning, because that is what fruit does. Standing
  food in winter went from 3,849 units to 492 — an 87 per cent cut and by a
  long way the largest effect measured in this project (t = -21). What it
  costs, and what it did not fix, is in ISSUES_FOUND.md #25

- 🚧 Food rots now, and it cost a fifth of the settlement. Every spoilage time
  in the tables was written as a day-count and stored at 1440 ticks to the
  day; the calendar was later put on a scale a life fits inside — twelve ticks
  to the day — and the food tables were not brought with it. So meat written
  down as lasting a day lasted a hundred and twenty of them, and grain written
  down as ten days lasted twelve and a half years. Nothing in this world
  spoiled, and everything followed from that: nobody ever went hungry, a
  larder was insurance against nothing, and six of the nine preparation states
  had never once been reached. Meat is now a season's business, fish rots
  faster than anything else anybody catches, berries do not see half a season
  out, and a dry seed keeps two and a half. Food left lying in the weather
  goes off three times as fast as food in a pack; a bowl or a basket between
  the food and the damp doubles what a pit is worth. And there is something to
  be done about it: laying food out dries it and hanging it in the smoke of a
  fire smokes it, which is twenty times and ten times the keeping. The cost is
  in ISSUES_FOUND.md #24 and it is not small — 52 people against 66 — and the
  reason is not the clock but what the clock exposed
- ✅ A settlement provisions. Nothing in this model had ever gathered *for the
  winter*: it gathered because it was hungry, ate what it picked in the same
  breath, and put away whatever happened to be left over. Probed directly in
  autumn, three agents in a hundred were carrying any food at all, so there
  was never a load to carry home — forty pits a world got dug and not one of
  them ever had anything in it. Three separate things were in the way, and all
  three were deadlocks rather than tuning. `Preparedness` stood behind
  `Sustenance`, so a forager could not store anything until it had solved
  farming, and food production is never answered in a people that does not
  farm — it sat below its threshold in eight agents out of eight for a whole
  settlement's life. What is in the pack in autumn is a harvest rather than
  supper, and carrying it home has to beat Hunger, which is a primary drive
  that wins every contest it enters. And `Cover` kept back three days' food
  while the agent was standing on its own larder, which refused 1,513 burials
  out of 1,525 for want of anything to bury. Winter stores went from nothing
  at all to 42 units standing through the lean season, pits from 2 to 10,
  burials from 4 to 86, and food dried or smoked from 2.8 a world to 666
- ✅ A people with nothing to build with digs itself in. `shelters built` was
  nought in every arm ever measured across the whole life of the project, and
  it was three deadlocks in a row: a tent wants eight wood and four hides,
  hides come off an animal and nothing else makes one, and hunting was
  unreachable. Unblocking hunting moved it from 2.25 shelters a world to 3.56,
  which is neither significant nor enough. What was missing is the shelter
  that depends on none of it — a hole in the ground with turf over it, costing
  earth and a morning, and worse than a tent in every way except that it can
  actually be built. It is dug rather than built, so it is its own verb:
  `build` is framing and wants poles in the hand, which is right for a tent
  and nonsense for a hole. `burrow` had been sitting dead in the matrix since
  the matrix existed; it is live now, and it finishes the subterranean family.
  Burrows dug went 0 to 39.6 a world and shelters standing 0.6 to 30.4, with
  people cold about half as often. Population and burials do not move, which
  is the honest reading: nobody was dying of exposure at ten thousand ticks,
  so a roof buys comfort rather than lives at this timescale. See
  ISSUES_FOUND.md #32
- ✅ Hunting is reachable at last. It had been put behind eating what you
  carry, behind foraging, behind walking to a known patch, behind moving the
  whole camp, behind walking back to ground that fed you once — and then
  behind being *desperate* on top of all that, so it was never reached. Forty
  agents in forty-seven believed it paid and none had ever done any, which is
  what a belief with nothing to update it looks like. The rule is narrow on
  purpose: a deer at your feet beats a berry patch twelve tiles off, and a
  deer across the valley is the expedition that starved two settlements in
  forty. Attempts went from 6.8 a world to 148, with population and burials
  unmoved — so it is reached and close to free, which is the honest reading
  rather than that it is now profitable
- ✅ A drizzle and a thunderstorm are no longer the same event, and a roof
  means something. What food lying out in the rain lost was a constant, and
  the intensity the weather has always reported was thrown away at the first
  comparison. Shade is the floor now and the open sky under a downpour the
  ceiling. A roof keeps the rain off what is under it and — cutting both ways
  — stops the sun drying it. That second half turned up a live trap: a default
  world puts exactly one building at the middle of the map, which is where
  several test fixtures stand somebody and drop food, so a passing test began
  failing ten runs in ten the moment a roof meant anything
- 🚧 Eating what will be lost before what will keep. `find_best_food_to_eat`
  weighted freshness alone, which is exactly backwards for a people with a
  store: a person eats the thing that is about to go and saves the thing that
  lasts, and that is the whole reason for preserving anything. It weights how
  fast a thing goes off as well now, so a dried strip is a twentieth as
  attractive as today's supper and exactly as attractive in February. **It
  made no measurable difference**, and the reason is that the diagnosis was
  wrong rather than the fix: what gets preserved does not sit in packs waiting
  to be eaten, it goes into the ground, and the store has been holding around
  a hundred units since the provisioning order was fixed. Kept because it is
  correct and costs nothing, and written up as a null. See ISSUES_FOUND.md #31
- ✅ Clay, and a fire that stops it being clay. `ResourceType::Clay`,
  `Pottery` and `Bricks` were three enum variants with nothing behind them:
  clay had been spawning on every riverbank and every marsh in every world
  since the project began and no agent could ever pick any of it up, because
  "clay" was missing from the vocabulary `Gather` answers to. That vocabulary
  turned out to be *two* vocabularies in two places that had drifted, and
  greens and roots had been going into packs as `"generic"` since the day they
  were added. There is one table now.

  Nobody is handed pottery. Every material in the chain is gathered by
  somebody who already wants the thing it makes, and nobody can want a pot
  before anybody has made one — so it is curiosity that fetches a handful of
  something nobody here has ever done anything with. A lump of clay holds a
  shape, which costs nothing but an idle afternoon to find out and is worth
  almost nothing on its own: an unfired shape holds nothing and comes apart in
  the rain. What it is worth is what a fire does to it, and that is a separate
  thing to find out, and bricks are a third — a people that has fired a pot
  has not thereby learned to make a wall. There is also an accident, which is
  the way most people got there: somebody sitting at a fire with clay in the
  pack loses a lump to the embers about one day in fifty, and in the morning
  it is not clay any more, and everyone round that fire saw it
- ✅ A map with danger on it. What an agent carried in its head had explored
  tiles, resource positions with an age and a source, buildings, storage and
  terrains — a real picture of the world's *things* — and nothing at all about
  danger. Somebody could be mauled at a ford and walk back to the same ford
  the next morning, because there was nowhere for "there are wolves in that
  wood" to live. It has a place, a name, a time and a strength now, and it
  fades: a pack works a wood for a season and moves on, so a fright is gone
  entirely after one, and a bad place is three tiles wide because "there are
  wolves in that wood" is not a fact about one tile. What goes on it is
  everything in sight that means harm *taken together* — one wolf is not much
  to a man with a spear and four of them are a different afternoon, and
  judging each separately would have him walk into the pack four times
  unafraid.

  It is load-bearing in two places. A patch of food in a wood where this agent
  saw wolves is further away than it measures, so a settlement works its safe
  ground first; and somebody running picks their way by what they know of the
  ground rather than bolting into the wood the pack lives in. **Population is
  up by a third (t = 2.4), and the reason is that a settlement had been
  spending nine thousand turns a world running away** — three per cent of
  every turn anybody took, at fourteen energy apiece — and agents who avoid the
  bad wood do not have to run out of it. That was not a number anybody had
  looked at, and it was the largest single waste left in the model. See
  ISSUES_FOUND.md #29 and #30
- ✅ A lesson about a situation rather than a hand-written string. `Lessons`
  has recorded what works since it was written, keyed on the thing attempted —
  `dry`, `gather:greens`, `hunt` — and every one of those keys was written out
  by hand by somebody who had already thought of it. So an agent could learn
  *that* gathering food does not pay and could never learn that it does not
  pay *in the spring*, which is why everything in this model that depends on
  when a thing works had to be a rule somebody wrote down: the bearing year is
  a table, sun-drying is a discovery flag, the fire that fires clay is a
  precondition in the executor.

  What is there instead is ten coarse facts about the afternoon — the sky, the
  season, a fire to hand, a roof overhead, water within a few paces, anybody
  else about — written down against every attempt anybody makes. Nobody names
  the situation, and nothing in the arithmetic knows what a season is. An agent
  works out which of them go with a thing working by comparing its record under
  one circumstance against its own overall record of the same thing, so a man
  who has only ever dried fish in the sun learns nothing whatever about the
  sun, and it takes one wet afternoon to teach him anything at all.

  A settlement arrives at about **two hundred and sixty** such lessons that
  nobody wrote down, and the strongest of them, reached independently by five
  sixths of the people in it, is **the bearing year** — gathering pays in the
  autumn and does not in the spring or the summer — which is a table in the
  world code that nothing had ever told an agent about. A people that has
  worked out the harvest **gathers a third more and puts a third more in the
  ground** (t = 3.4 and 2.6, thirty-two worlds a side). It costs a fifth more
  refused turns, which is established and written up rather than buried: they
  gather very hard in the autumn and a good share of it finds a patch already
  picked out. See ISSUES_FOUND.md #33
- ✅ A place that has run out, and somebody who knows it has. The map an agent
  carries knew *what* was at a place and never whether there was any of it
  left, so somebody would pick a patch bare, walk home, and walk back to the
  same bare ground the next morning. "No food sources nearby" was **ten
  thousand refused turns a world** and "inventory full" another five thousand
  — between them more than half of everything a settlement ever got refused,
  because several of the paths that produce a gather cannot see the world at
  all. Stripping the last of something goes on the map, and it is not a private
  fact: everybody standing near watches the ground go bare. It fades after half
  a season, because a patch picked out in June is bearing again by September.
  And a gather that could not come to anything is refused on the way past
  rather than after the turn is spent. **Refusals more than halved, from nine
  per cent of every turn to under four** (t = -11.1). See ISSUES_FOUND.md #34
- ✅ Curiosity as a question whose answer arrives later. "What happens if I
  leave meat in the rain?" is not a turn: it is a thing put down, a state
  remembered, and somebody walking back in a few days to look. Every other
  kind of curiosity here answers in the turn it was spent, which is right for
  a lump of clay and wrong for most of what a stone-age people has to find out.
  The one branch that reached for the later kind was gated on the sky being
  clear — the code already knew the answer and only let anybody run the
  experiment on the days it comes out well.

  What is remembered is what the thing was like and **what the sky was doing at
  the time**, carried rather than looked up on the way back, because by then
  the rain has stopped. Coming back to find it changed is the lesson; coming
  back a week later to find it exactly as it was left is also a lesson, and the
  one that stops a man repeating a pointless thing for life. A settlement puts
  and answers **a hundred and eighty-eight such questions** that nobody
  arranged, at no measurable cost. It is at the edge of what the pattern
  arithmetic can use rather than past it, and that number is in the write-up.
  See ISSUES_FOUND.md #35
- ✅ Counting the waste. The point of preserving anything is that the time
  spent getting it was not wasted: **if half the meat rots before it is eaten
  then half the hunt was wasted**, and an hour spent hunting is an hour not
  spent doing anything else. Nothing in this project had ever counted that —
  every preservation change for a dozen entries was judged on how much was
  *in* the store, which measures activity rather than whether the activity was
  any use. Food goes off in a pack, in a pit and where it lies, and all three
  simply deleted it. Counted in all three now, and the number nobody had is
  that **a settlement throws away a quarter of everything it gets**: 3,382
  units eaten against 1,135 rotted. See ISSUES_FOUND.md #36
- ✅ Burying and salting as questions, and somebody to ask. The other verbs put
  questions the same way leaving a thing out does, and **the verb decides what
  counts as a good answer** — a thing left on the grass that is unchanged a
  week later teaches nothing, and a thing *buried* that is unchanged a week
  later is the entire point of burying it. Each question knows where to look:
  a hole, a pack, or the grass. Firing clay was already a same-turn experiment,
  so what was added is the version that is genuinely a question — a lump left
  at a lit fire is not a lump of clay in the morning.

  And somebody carrying a thing you have never seen the like of can now be
  asked about it, under Curiosity, which is to say only when nothing worse is
  pressing. They have to actually understand it, and what passes between them
  is the *name of the discovery* rather than a belief — so being told lets the
  hearer go and try it, and trying it is what decides whether they believe it.
  Nothing anywhere had ever let a man who worked something out tell anybody: a
  settlement of forty could work the same thing out forty times over.

  Questions asked and answered more than trebled (196 → **661**, t = 17.8) and
  **a quarter of a thousand discoveries a world now pass from one head to
  another** (t = 22.8). Nothing else moves, and the reason is worth knowing
  rather than shrugging at: this people's one load-bearing discovery already
  reached 36.6 of 37.1 people by being *watched*. Telling is redundant for the
  only thing worth telling — until there is a discovery that does not announce
  itself to everybody standing nearby. See ISSUES_FOUND.md #37
- ✅ What you cannot carry stays where it fell. `add_item` enforces the weight
  limit and returns `false`; butchering ignored what it returned, so a kill
  bigger than a hunter could carry **stopped existing** rather than being left
  in the field. It stays where it fell now, and it counts.

  The interesting half is what that makes of preserving. **Drying takes the
  water out, and water is most of what meat weighs** — dried meat is a third
  the weight of the meat it was, so a hunter who dries a kill before walking
  home carries more of the animal home. Preserving buys carrying capacity as
  well as time, and they are the same thing from different ends. Salting buys
  the keeping and not the carrying, because salt puts back about what it draws
  out. A leather bag holds more than a flax basket and costs an animal and a
  leatherworker, because carrying capacity is what this people is shortest of.
  **A settlement eats an eighth more than it did (t = 2.6) and wastes a
  quarter less of what it gets, 73% used to 77% (t = 3.0).**
  See ISSUES_FOUND.md #38
- 🚧 And where the food actually goes, which is not where anybody was looking:
  per settlement per ten thousand ticks, **537 units rot in the pits**, 438
  where they lie, and 231 in packs. The larder is the biggest single source of
  waste in the model — it gets filled and never drawn down. Every "winter
  store" headline above is measuring a stock that quietly loses about half of
  itself. Measured and written up, not fixed here — the fix, and the wrong
  answer measured first, are two entries below. See ISSUES_FOUND.md #39
- 🚧 Making the trip pay, and a vessel nobody had ever wanted. `what_i_would_make`
  asks only after **tools**, so a carved bowl and a fired pot both declared what
  they hold and neither was ever made on purpose by anybody. No agent could
  carry water, so every drink was a walk to the river; `Boil` was refused for
  want of something to hold the sea in 250 times a world, putting salt out of
  reach; and the fluid family built in an earlier batch *because vessels
  existed* has been inert ever since. Two older faults sat underneath: carving
  a bowl wanted discovering where weaving a basket is obvious, and a fired pot
  was set to hold **exactly** what a wooden bowl holds, with a comment above it
  saying "a little more than a carved wooden bowl".

  The other half is taking what you can carry while you are standing there
  anyway — the trip is the expensive part and the load is nearly free, so
  somebody on a salt flat fills up rather than taking what they need today.

  **Burials are down a fifth (t = -2.7), and that is the only established
  result.** The vessel half is built, tested, and **does not reach the field**:
  vessels per settlement did not move and boil refusals did not move. The
  diagnosis and the next move are written up rather than guessed at, along with
  two self-inflicted regressions caught by measurement and a third instance of
  this project's recurring vocabulary defect — salt, greens and roots all
  existed as item types and none was in the table that lets the world price or
  store a thing. See ISSUES_FOUND.md #40
- 🚧 The order of a list, and what was actually wrong. `what_i_would_work_on`
  took the **first** thing in the working table it could do and stopped, so
  whatever sits early and has materials to hand won every turn for every agent
  for ever — the same trap already found and fixed once in the function beside
  it, and never carried across. Each agent starts at its own place now, worked
  out *before* the belief is consulted so that a man's trade does not change
  by the hour.

  Leatherworking is scraping a hide, not cutting one: a flint takes the hair
  off and turns skin into leather, where cutting gets you two smaller hides.
  Sewing a bag out of the leather afterwards is crafting — the skill sits one
  step earlier and putting it on both paid a man twice for one trade.

  **And the previous entry's diagnosis was wrong.** The block on vessels was
  not the table order. Counted directly, of thirty-one people: **26 wanted a
  vessel and could make one, 28 held the wood, and 4 owned anything to carve
  with.** The list of trades a pair of hands wants to be equipped for held
  hunting, woodcutting and leatherworking — and nothing anywhere else in the
  model ever wanted a tool, so no crafting or mining tool was ever made on
  purpose. Vessels are up 29% (t = 1.8) and **nothing is established**; a
  settlement still does not make vessels. What is established is why not.
  See ISSUES_FOUND.md #41
- ✅ Every wasted craft turn in the model, gone. `Craft` was refused almost
  never and simply **never attempted** — 110 taken against 1,896 workings in a
  world — because the tool a man wants and the material that tool wants both
  sat behind the undirected "work whatever is in the pack". Naming a step now
  checks that the step can actually be taken (a fire where a fire is wanted, a
  hammerstone owned where one is wanted), and the tool-getting-out machinery
  has been taught about a recipe's own tool, which the verb matrix cannot
  express. **Refusals went to exactly nought in all sixteen worlds**, against a
  mean of 68.6, at no cost anywhere.

  The reverted half is the more useful finding. Putting the directed wants
  ahead of the undirected working — "being equipped before pottering", which
  sounds obviously right — cost a settlement **two thirds of its vessels**
  (t = -4.6). **The pottering is where bowls come from**: carving a bowl is a
  *working*, so that branch is the only route to a vessel anybody takes.
  Demoting it deleted what it was producing rather than redirecting the turns.
  A branch that looks like idling may be the sole producer of something.
  See ISSUES_FOUND.md #42
- 🚧 The effort economy is decorative: nobody in this model is ever tired. A
  specification arrived describing tools that make work faster, an agent
  weighing *"eight hours with this axe, or two hours making a better one and six
  with that"*, preparation cascades and specialisation into trades. The first
  piece looked obvious, so it was built first: `Tool::how_much_better`
  multiplies what comes *off* a job and touches nothing else, so a stone axe and
  a bronze axe fell a tree at the same price and a pit costs a flat 22 energy
  whether it is dug with an axe or with bare hands. `what_this_job_costs_me` is
  the other side of it, applied in one place, with seven tests.

  **It measured null.** 32 worlds a side: alive t = 0.67, eaten t = -0.17,
  deaths t = 0.65, pits t = -0.50, not one column significant and two drifting
  the wrong way. Then one probe, 45,000 samples of a living agent's energy:
  **mean 96.6 out of 100, 97.2% of samples above 80, and nothing in a settlement
  ever below 40.** Eating restores `amount * 20.0` capped at 100 — a meal of
  five units refills the whole pool — and `Eat` is 9.85% of every turn. One meal
  pays for four pit-diggings.

  So forty-odd tuned `with_energy_cost` constants, including one commented as
  *"the most expensive single act in the model"*, charge against a pool that is
  always full. **Reverted rather than shipped inert**, and filed as #200 with
  the probe: either effort binds or the model should stop pricing in it and use
  the turn, which is what the specification actually means by hours.

  It also relocates the specification's central idea. Almost every action takes
  exactly one turn whatever it is done with, so a tool's *yield* multiplier
  already is the time economy — four wood a turn instead of two is eight turns
  instead of sixteen — and that part works. What has no equivalent anywhere is
  the reckoning that compares them (#194). The rest of the specification is
  filed rather than guessed at: #193 #195 #196 #197 #198 #199. See
  ISSUES_FOUND.md #70
- ✅ Every man knew how to make an axe and one in thirty-five owned one. "They
  struggle to complete simple tasks" — so the first thing built was an
  instrument for where a settlement's day goes, and the first thing it did was
  kill my own first guess. `Move` is **42.7% of every turn**, which looks like
  the whole answer; counting every unbroken run of walking says **79% of things
  done need no walk at all** and the mean is **0.71 paces per thing done**. The
  walking is fine.

  It is the tools. `Work` was refused **88.2%** of the time, `Excavate`
  **99.4%**, `Hunt` for want of a spear 2,227 times — and nearly every refusal
  was one refusal: *nothing in hand that is any use for this*. Then: of 181
  people alive, **all 181 knew how to make a handaxe, a stone knife and a
  spear; five owned an axe and nineteen owned a knife.** Crafting was not
  broken — it succeeded every time it was attempted, and was attempted 270
  times a world across forty-five people over ten thousand ticks. `Craft` sits
  in the Utility branch behind two others, and Utility rarely wins against
  Hunger.

  This project had already written the answer for the *other* half of the
  problem: *reaching for a tool is not what somebody does with a spare moment,
  it is what they do just before using it.* Nobody had done the same for
  **making** one. `make_what_this_wants` sits beside `get_the_tool_out_for`:
  when the verb matrix is about to refuse an action for want of a tool, and the
  agent knows a step towards that tool it could take now, it takes the step.
  The turn was lost either way. It asks the same function the executor asks, it
  only names a step that can actually be carried out, and it checks the
  substitute for short-handedness before taking it.

  Measured 32 worlds a side: **people carrying a knife 3.9 to 8.3** (t = 7.5),
  **vessels 14.2 to 22.1** (t = 3.9), **pits dug 5.5 to 7.8** (t = 5.9), crafts
  +24% (t = 5.1), short-handed refusals **-37%** (t = -9.2), failure rate
  0.0230 to 0.0212 (t = -3.8). The digging row is the shape of it: **half as
  many attempts and 43% more pits.** Survival moves the right way and is not
  significant on its own — alive +2.7, eaten +283, deaths -1.9.

  Three things the instrument found and this does not fix, each filed with its
  numbers: 1,690 short-handed refusals a world remain where the *material* is
  missing (#190); **`TrySwapping` is refused 100% of the time, 6,489 attempts
  and not one success** (#191); and `Examine` is refused 92% because an agent
  re-examines what it has already learned nothing from (#192). See
  ISSUES_FOUND.md #69
- ✅ A place, a date, and how much was on it — and being right, which nothing
  recorded. Everything one agent could tell another was a position, a resource
  type and a date. A listener already weighed the *age* of a claim and had no
  way at all to weigh either against **"the last handful of a worked-out one"**.
  The remembered amount now travels with the sighting: in a man's own map, in
  what he tells people, and in `SpatialMemory::value`, a field that has existed
  since the model had memories and was `1.0` for everything, so a spring and a
  puddle were remembered alike.

  The measurement that mattered was not the one I set out to take.
  **`correct_count` was zero across thirty-two worlds** against 1,646 wrong
  ones: nobody in a running settlement had ever been recorded as having told
  the truth. Both copies of the verification sweep call `hearsay_in_view`,
  which filters to claims where the ground is *bare* and is incapable of
  returning one that held up, so a man's standing could only ever fall.
  `hearsay_borne_out` is the other half and both sweeps call it now — **0 to
  19,494 a world** (t = 21.1). That is also half of what #185 wants: "true
  statements strengthen trust" needs true statements to be recorded.

  And an honest report of a poor place is safe to make. Bare ground had two
  excuses — stale news, or somebody stripped it first — and now has a third:
  **he did say it was nearly gone**. It cannot shelter a liar, because a liar
  claims twenty and the excuse stops at three. Accusations fall about a quarter
  (51.4 to 35.8 times caught out) and that is **not significant at thirty-two
  worlds a side**; the direction is right and no more is claimed.

  **The obvious half is not shipping.** Walking to the place you remember most
  of, rather than to whichever is furthest off, produced one world in each of
  three arms that refused for want of water **3,092, 851 and 13,004 times**
  against a baseline worst case of seven — and weighing the amount by staleness,
  the obvious guess, made the worst of them worse. Reverting that one branch put
  the worst world's failure rate back to exactly the baseline's. Held back with
  its numbers as #189. See ISSUES_FOUND.md #68
- ✅ Two names for one action, and the two claims it cost. The entry below
  ended by wondering whether shy animals had retired the threat tree, and the
  answer to that was in ten lines of the function it accused: `shy_away_from`
  already exempts every Aggressive and Territorial species, which is every
  predator in the world. What the instrument found instead was worse.

  `actions_taken` booked everything chosen in the fear branch as "Flee";
  `actions_failed` booked by the action's own name. So a run that happened and
  a run that was *refused* went into different buckets, and `Freeze` — chosen
  in the same branch — read as never once taken. **Sixth appearance of this
  project's duplicated-vocabulary defect, and the first to corrupt a published
  measurement rather than a behaviour.** The decision reached `FleeFrom` 1,558
  times in four worlds while the tally recorded none.

  **Two claims withdrawn.** `Freeze` at "zero in sixty-four worlds" is wrong —
  twelve baseline worlds take it **10,971 times**, in the same rare-catastrophic
  shape as the refusal itself. And "running happens nine times as often" is
  withdrawn as unresolved: re-measured with the label fix on *both* arms it goes
  the other way (1,256 to 363 a world, t = -2.79), a second draw goes the
  original way, and per-world flee counts run from 1 to 7,365 — the quantity is
  too skewed for a mean of thirty-two to mean anything.

  What #176 did do, measured on something that is not skewed: **ground put
  between a man and the thing rises from 7.2 to 16.3 paces a run** against an
  intended bolt of nineteen. The old clamp turned a landing off the map edge
  into a one-pace shuffle. And freezing falls from 10,971 to 1,092 over twelve
  worlds — the cornered case fixed, not the branch dead.

  The tree is asked on **1.45% of turns** and 80% of what reaches it falls out
  at "nothing named", which is it correctly declining a quarrel between people
  and handing over. Its two halves are quiet for two different reasons, and
  only one is a defect. A wolf put one pace from a healthy adult **leaves at
  six paces a tick and does not come back** — the fauna model reading the odds
  — so nothing gets near enough to frighten anybody, and creature fear runs at
  0.15% of turns. What does stay near is what a person can beat, so it is
  appraised as anger, at 9.8% of turns. And that anger cannot pass its own
  gate: it caps at 0.5 and `should_attack` wants more than 0.5, so an agent
  turns on a wolf only because it also resents a boar. Left open as #188. See
  ISSUES_FOUND.md #67
- ✅ Two words for one question, and a man who could not get off the beach. The
  largest single refusal this model has produced — **76,644 in one world**, three
  quarters of every turn taken in the settlement — and the cause was neither of
  the two things the report guessed at. Two things asked whether there was
  anywhere to run: the decision tried three directions at **three paces**, the
  running tried the same three at **nineteen**. Between those numbers sits a
  shoreline. A man three paces from open water with the thing inland has
  somewhere to go at three and nothing but water at nineteen, so the decision
  said run and the running said "Nowhere to run" — and nothing about the next
  turn was different, so it said it again, for the rest of that agent's life.
  The project's duplicated-vocabulary defect for the **fifth** time.

  One function answers it now, and both callers ask it. It tries **eight ways
  out rather than three**, each at the full bolt and then at every shorter
  distance down to a single pace, so a narrow gap counts as a gap; and where
  there genuinely is nowhere, **standing your ground is an answer that costs a
  turn** rather than a refusal that repeats forever.

  Measured 32 worlds a side: refusals **19,626 in the worst baseline world to
  zero in every world of the arm**, worst-world failure rate 9.8% to 2.6%, and
  **running actually happens nine times as often** (0.25 to 2.22 a world,
  t = 2.48) because the decision's yes is now one the running can act on.
  Everything else null, which is what a fix to a rare tail should look like.
  **The "running happens nine times as often" figure and the `Freeze` figure
  that were here are withdrawn — both were read off a miscounted tally, and the
  entry below corrects them.** See ISSUES_FOUND.md #66
- ✅ There was no sink. The settlement was living on food that never aged. The
  entry below held the food-clock rule back because eaten plus waste fell from
  12,874 to 6,692 with it on, and concluded six thousand units were leaking.
  **Wrong.** The obvious instrument — sum every unit of food anywhere once a tick
  and compare what leaves against what is booked — was built last instead of
  first, and says food is conserved to **under a hundred units over six thousand
  ticks**. Nothing leaks. The settlement acquires less: `Gather` falls a third
  and the difference goes into preserving, with `Dry` rising from nowhere in the
  top sixteen actions to 1,678 and burying up 76%. Before, a stack's clock was
  whatever the first thing in it had, so a pack topped up all day never aged —
  a people was living on food that could not go off.

  Four hypotheses were measured and each was wrong, and two of them were real
  bugs worth fixing anyway: `add_item` added only the incoming item's weight
  while merging could change what the whole stack weighs, so a stale
  `current_weight` could silently destroy food (worth ~9 units a world); and
  `more_food_than_he_will_get_through` asked `is_food`, which is true of mould,
  so a man with eight units of rot declined to forage — the same mistake #43
  fixed once already, made again one entry later. A pit also now puts this
  autumn's load **beside** last autumn's rather than tipping it in on top.

  Shipped, with the cost stated plainly: **a settlement is a sixth smaller and
  eats less than half as much (t = -8.3), and wastes a quarter of what it used
  to** in packs (1,532 to 388) and on the ground (1,438 to 640). The model's
  central quantity halves. It ships because the alternative is a pack that lies
  to the agent reading it, and every decision this simulation exists to make is
  made off that reading. See ISSUES_FOUND.md #65
- 🚧 A thing rebuilt from its name is not the thing. Four places took an item's
  *name* and count and constructed a fresh item, discarding everything else
  about it. **Giving** somebody a week-old fish handed them a fish that would
  never go off, and gave away a dried strip as undried. **Theft** did the same.
  **Harvesting a plant** attached no food clock at all. And the **stack merge**
  let a lot with no clock swallow a real one — an item with no food data never
  rots, which is where the immortal food in ISSUES_FOUND #45 came from. All four
  fixed, and measured null together.

  The clock rule — fresh food tipped into a basket going over comes down to meet
  it, because mould spreads — was held back for one commit on the reading that it
  lost ~6,000 units from the ledger. **That reading was wrong and the next entry
  corrects it.** See ISSUES_FOUND.md #61
- 🚧 Making food scarcer does not make a people careful. Thinning what there is
  to gather and capping what one person takes, done together and measured
  separately. **Mostly a negative result, and the negatives are the valuable
  part.**

  The first attempt measured nothing at all, because **there are two resource
  spawners** and berries come out of the one that was not being changed — the
  fourth instance of this project's duplicated-vocabulary defect and the second
  in two batches. Both read one table now, and a patch is the size the ground
  under it will carry. With that fixed the thinning is real: a world's edible
  standing crop goes from **7,413 to 3,944**.

  And it still changed nothing — **not one column of thirty-two worlds a side
  reaches significance**. The standing crop turns out to be a buffer: a patch
  regrows at a fixed rate until it hits its cap, so halving the cap makes it top
  out sooner and produce at exactly the same pace. That is the springs lesson
  from the entry above, arrived at from the other side. Wild animals now get out
  of a person's way — nothing in the fauna module knew agents existed except the
  predator pass — and **hunting is 250 actions in 270,000**, so it cannot matter.

  So the flow was halved instead, and **reverted**: the population did not move
  and **efficiency went from 0.74 to 0.70 (t = -3.0)**, with more rotting in
  packs and more left on the ground. People ranged further, carried more when
  they found anything, and lost more in transit. The waste in this model is a
  behaviour, not a supply artefact, and starving people does not fix a
  behaviour. See ISSUES_FOUND.md #57
- ✅ A spring is a flow, not a barrel. The entry above raised the rate; this is
  the half that matters. Water was a `ResourceNode` with an `amount` that
  drinking decremented — a stock — and **a spring does not have a set amount of
  water in it**. It recharges, and what limits what you can draw from it in an
  afternoon is its rate. A source now cannot be drawn below what it puts out:
  at eighty founders and a hundred and forty-one people alive, a world keeps
  two thirds of its water and **the emptiest source still holds twelve**.
  Nothing can be drunk to nothing at any population.

  And thirst is now a reason to leave a country. `migration_action` read the
  Hunger drive and nothing else, so a settlement whose springs had gone dry and
  whose hedgerows were full had no reason in this model to move, and did not —
  which is why nobody migrated. There is a constant called
  `HOW_FAR_A_PEOPLE_WILL_MOVE`, documented as "how far a people will pick up and
  move for water they can count on", which was used only by a food-seeking
  branch.

  Two things measured wrong on the way and are recorded rather than quietly
  fixed: the first springline made **rivers undrinkable** (running water's flow
  is deliberately bigger than its bed), and exempting springs from the
  picked-out memory — which looks obviously right, since a spring is running
  again in ten ticks — put the failure rate **up** rather than down, because
  remembering the spring was low is what sends a man to the next one.

  What fixed the last of the cost was a physical reading rather than a number:
  the pool is what has gathered and the springline is what is *arriving*, so a
  source at its springline gives a mouthful taken from the flow and the pool
  does not move. A queue at a spring all get a drink. Measured at thirty-two
  worlds a side: **the failure-rate regression goes to nothing (t = 0.01)** and
  every other column is null, which is the right result for a batch whose whole
  purpose is that the world should stop doing something impossible.
  See ISSUES_FOUND.md #53
- ✅ Three "known flaky" tests, and two of them were not flaky. The suite had
  twenty tests on a list of intermittent failures. Run on their own,
  `water_is_not_used_up` failed **twelve times out of twelve** and
  `honest_agents_do_not_end_up_accused` **eight out of eight** — standing,
  reproducible failures filed as flakes and left. Each had a real defect under
  it.

  **A settlement drank its own springs dry.** With nobody in the world the
  water total holds at 100%; with twelve founders it fell to 55%, and eight of
  a world's twenty-one sources sat at two units out of four hundred. The
  comment beside the numbers said "running water: whatever is drawn is replaced
  from upstream" and the number under it gave back 0.15 a tick against a camp
  drinking three. That also explains a figure that has been in every refusal
  table in ISSUES_FOUND without being read: **"no water sources nearby" was the
  single largest refusal in the model** — a settlement standing among its own
  dry springs. A stream is a flow now, not a stock.

  **An honest man was called a liar for two different reasons.** A theft was
  filed in the same column as a proven lie, and being honest is about what a
  man says rather than what he takes. And worse: a mined-out mineral seam is
  deleted from the map, so honestly reporting a clay seam somebody else strips
  overnight left ground indistinguishable from the invented spot a liar names.
  The world remembers worked ground now. Fixing the first of **two copies of
  the verification sweep** took the count from 19 to 10 and no further, which is
  how the second was found — the fourth instance of this project's duplicated
  vocabulary defect.

  **And a cluster of three was usually one.** `spawn_resource_clusters` placed
  each node at a random offset and silently dropped any that landed on the
  wrong terrain, one throw each. Clay wants riverbank, which is a ribbon two
  tiles wide. Asked for five clusters of three, a world made **5.8 nodes**;
  it makes 13.5 now. Every clustered resource went through this.

  Measured at thirty-two worlds a side, with the clustering fix backed out
  again to separate it: **the failure rate falls in both arms** (t = -4.1 and
  t = -2.8), which is the water fix doing what it was for. But putting the
  world's resources back to the number the config always asked for **costs
  eight points of efficiency** (0.82 to 0.74, t = -8.5) — doubling what there
  is to gather does not double what anybody eats, it doubles what rots in a
  pack. That is #43 one step upstream: the larder now asks what the camp will
  eat before winter, and gathering asks nothing of the kind. The next thing.
  See ISSUES_FOUND.md #46, #47, #48, #49
- ✅ The larder was four years deep. Entry #39 named the store as the biggest
  single leak in the model and left the question open: do people draw on it too
  rarely, or is the pit's rate wrong? Neither. What is in the pits is almost all
  *dried* food in *lined* pits, the best this model can do — and a pit takes 300
  where a settlement eats about a hundred in a winter, so "is there room in the
  hole" was never once the binding question. A people buried until the ground
  held four years' eating and went on burying. Everything past the first winter
  was going to rot whatever its rate was.

  Burying now asks whether there is already a lean season's eating in the ground
  for the people about — a thing somebody standing in their own camp can see —
  rather than whether the hole has room. **A settlement eats two and a half
  times as much food, 3,750 units to 9,831 (t = 8.5), and carries eight more
  people, 39.1 to 47.2 (t = 3.0).** Rot in the pits is down (t = -3.4), burying
  is halved (t = -8.9), and efficiency goes 76% to 82%.

  The wrong answer was measured first and is the more useful half. Asking the
  store *before* going out for food — a hole full of supper underfoot ought to
  beat a walk to a berry bush — draws on the store five times as often and
  halves the rot, and **costs a fifth of everything anybody eats (t = -3.1) and
  six of the people (t = -2.5)**, with efficiency not moving at all. A meal out
  of a hole costs two turns where a berry costs one, and nearly everything taken
  out had been buried by somebody a day earlier. Reverted, with a test standing
  on it.

  Three defects underneath: a fourth instance of this project's circular
  precondition (`Cover` leaves you one meal, and one meal was enough to lock you
  out of the store you had just filled); a count that called an uncut haunch
  "food", so a man carrying a rotten carcass read as provisioned; and a
  starvation loop — a pit offered an uncut haunch to somebody who could not eat
  it, over and over, and **one settlement in sixteen starved to death standing
  on its own larder**. See ISSUES_FOUND.md #43
- ✅ You cannot eat a deer. Agents ate raw flesh in two-kilo lumps with nothing
  in the way: one `Eat` swallowed one unit off a kill, the only gates were
  "is it spoiled" and "is it poison", and cooking was worth 2.7 times the
  nutrition and nothing else. A carcass is now whole until somebody takes a
  knife to it, and while it is whole it can be neither eaten nor put over a
  fire. Everybody is born knowing a carcass comes apart — there is nothing to
  discover about a joint of meat — but knowing it is not the same as having an
  edge to do it with. Strips come off a joint rather than off the animal, dry
  in two days where a joint takes most of a week, and twice as many of them
  fit over a fire, which is the whole reason anybody bothers cutting a thing
  thin rather than just quartering it
- ✅ People get ill. There was no sickness anywhere in this project: the only
  health consequence in it was a flat ten damage for eating something already
  past saving, taken in one tick and done with, so a settlement could live on
  raw flesh and sleep in its own midden and never know the difference. Raw
  flesh is a gamble now — about one meal in twelve — and food that has started
  to go is a worse one the further gone it is, and a day on fouled ground is
  worse again. A body fouls the ground it falls on, which is what makes a
  corpse a thing to be away from rather than a nutrient deposit. An ailment
  lasts days rather than landing in a tick, because what costs a settlement is
  not the damage but somebody laid up for a week in the autumn. Two weeks in
  bed off the same thing and an agent will leave that thing alone, unless it
  is starving, in which case a strong enough survival drive overrides the risk
  as it does everywhere else
- ✅ Salt, and the sea it mostly comes out of. `PreparationState::Salted` was
  written, tested and unreachable for the whole life of the project, because
  there was no salt anywhere in the world — and there was only one kind of
  water in it: a river, a spring and the sea were the same terrain and the
  same drink. There are three new grounds now. The sea forms where the land
  falls furthest away, salt marsh where it meets the shore, and salt flats
  where a shallow sea dried up and left what was in it. Salt is picked up off
  a flat, broken out of rare seams in the hills, or boiled out of the sea by a
  people who have neither — and a pot of the sea leaves almost none of it,
  which is why salt is dear. Salting keeps food about seven times as long and,
  unlike drying, needs neither a fortnight of sun nor a fire kept going, so it
  is the answer in a wet autumn. Sea water is a drink that costs more than it
  gives: it slakes the thirst on the tick and raises it for days afterwards.
  Everybody knows better — a mouthful tells you what it is — and nobody who is
  three days dry knows better at all.

  What all three cost is **cooking, down by three fifths**, and that is the
  efficiency trade again: turns spent quartering a deer, boiling the sea and
  rubbing salt in are turns not spent at the fire. Population and burials do
  not move. Four rounds of measurement were needed and the third and fourth
  each found something: a settlement's entire preservation output had been
  drying *whole fish*, which ought to rot; and with every way of preserving a
  thing placed ahead of burying it, a settlement spent two thousand turns a
  world preserving food and put a third as much in the ground as before any of
  it existed. See ISSUES_FOUND.md #28
- ✅ The sun and the rain do the preserving, and somebody has to notice. Food
  lying on the ground used to age at one flat penalty whatever the sky was
  doing. Now what falls on a thing decides what happens to it: rain rots, and
  sun either dries a thing or ruins it depending on whether it is thin enough
  to dry through. A whole fish left in the sun goes off; the same fish opened
  out and cut into strips dries, and dried keeps twenty times as long as raw.
  Berries, greens, grain and roots dry as they are. Shade is the middle case
  and still costs something, because nothing keeps outdoors.

  Nobody here is born knowing any of that, which is the point. `cut fish` and
  `cut meat` are obvious workings — anybody with an edge works out that a fish
  comes apart — and worth exactly nothing on their own. The value is entirely
  in what the weather does afterwards, which nobody can predict and everybody
  can watch. An agent carrying more than it can eat, with no store within
  reach and a clear sky, puts it down; that is an ordinary thing to do and it
  happens to be the beginning of every preserved thing this people will ever
  own. When the world turns something from raw to dried, everyone within six
  paces is told, the same way the four routes into farming work — and an agent
  that has never seen it happen cannot choose to do it. Watching food go off
  in your own pack now costs worry rather than nothing, because what has been
  lost is not the meal so much as the certainty of the next one.

  Leaving that last gate out of the *decision* and putting it only in the
  executor cost more than half the store — agents spent their turns choosing
  an action that came straight back refused, and winter stores fell from 42
  units to 17. With the check where it belongs, winter stores go from 42 to 84
  and burials of food from 83 a world to 498, on a population and a death rate
  that do not move. Deliberate drying fell by nine tenths and that is the
  system working: the preserving happens in the weather now, and what an agent
  contributes is cutting the fish up and leaving it somewhere sunny. Salting
  is still unreachable because there is no salt in this world — see
  ISSUES_FOUND.md #27
- 🚧 A hole in the cold ground, which is a larder. It pays for itself now that
  anything reaches it — A settlement had nowhere to put anything it ate: what a person could
  put by explicitly excluded food, and the only place to put anything was a
  single global bag of counts with no position that nothing ever spoiled in.
  A pit is dug with a stone tool, food goes in, the earth goes back over it,
  and what is under there ages at a quarter the rate it would in a pack —
  which is not a cellar, but it is the difference between eating what you
  found today and eating in February. Hunger draws on the nearest store before
  it walks out to a berry bush. Two rounds of measurement each turned up a
  real defect and each was fixed: the first cut foraged for the larder all
  year round, which is nobody's idea of husbandry and cost 351 trips a world,
  and the second asked for a pit wherever somebody happened to be standing,
  so ninety-eight attempts in a hundred were somebody trying to dig a hole in
  a lake. What is left is a settlement that digs about one and a half pits and
  keeps a dozen units in them, at a cost of some seven people (se 5, not
  significant, but negative in sign across all three arms). It does not fix
  what it was built to fix, because nothing in this world makes anybody go
  hungry — see ISSUES_FOUND.md #21 and #23
- 🚧 Laying down your life for your own, which half works. A wolf standing
  over somebody you love who cannot deal with it themselves brings you at it
  whatever the odds — the one place in the model where an agent knowingly
  takes the worse of two options, and the only way a parent can die for a
  child. The other half, going without food you need so that somebody who
  needs it more gets it, is built and tested and has never once fired: at ten
  thousand ticks not one of sixty-five agents is carrying any food at all and
  not one is starving. Everything gathered is eaten within a few ticks. There
  is no occasion for the sacrifice because there is no scarcity and no larder.
  See ISSUES_FOUND.md #21

- ✅ Two hands, that hold particular things. `A_PAIR_OF_HANDS` had been in the
  matrix since the matrix existed and nothing had made it true: a tool in the
  pack was a tool in the hand, so an axe helped you the moment you owned one
  whether or not you had got it out, and "a free hand" could only be guessed
  at from how loaded the pack was. Getting the thing out is a turn's work now
  and is worth it, a person can only hold two of the four or five tools a
  working settlement owns, and a job that wants a hand free gets one by
  putting something away rather than by failing. Carrying costs something too
  — up to about twice the energy per step at the limit of what the arms will
  hold, where before a man walked as easily under sixty pounds of stone as
  under nothing. What this batch is actually worth, though, is the bug it
  turned up: because the free-hand test was a pack-weight test, and a
  settlement lives at the limit of what it can carry, every action the matrix
  said wanted a hand free was being quietly refused for everybody. Removing
  that one test on its own takes a settlement from 65.6 people to 79.8

- ✅ Taking what is not yours, and running. Both were in the world already and
  neither had a name: flight went out as a `Move` like any other, so a man who
  had escaped four wolves had no record of having escaped anything. Running is
  its own verb now — further in a turn than a walk and a good deal more tiring
  — and it taught us something the moment it was measured. Filing an escape
  under fighting, on the grounds that they are two answers to one question,
  made the settlement pick nearly three times as many fights: a man who had
  outrun four wolves came away believing he could beat the fifth. Getting away
  is its own lesson now, and the attack count went straight back to where it
  had been (97 a world to 40, against 35 before any of this). Theft is the
  other half and is very nearly a null result: it is built, tested and chosen
  once in eight worlds of ten thousand ticks, because a band of forty who all
  grew up together has nobody standing next to it that it distrusts. The
  machinery is there for a world that has strangers in it; this one does not

- ✅ Things done with a vessel of water, which is the family that was entirely
  declaration until somebody could hollow out a bowl. Flax left in water lets
  go of its fibre and gives three times the cordage — cordage carried per
  settlement went from 31 to 46. Fruit and water left alone turn into
  something that keeps a fortnight where berries keep hours, which is the
  storing the specification asked for. And a pot of flour and water over a
  fire is bread: whole grain improves in the embers and ground grain turns to
  ash, a distinction the food tables already drew and nothing had ever used
- ✅ A basket, a bowl and a handful of flour. Flax woven into a basket is how a
  person carries more than their arms hold. A block of wood hollowed out is
  how water travels — and it is what the container machinery in this codebase
  had been waiting for since it was written, because nothing in the world had
  ever made one. Grain between two stones gets a third more out of the seed
  and keeps rather less well, which is why you grind it when you mean to eat
  it rather than when you bring it in
- ✅ Looking closely at a strange thing you are already carrying. The third
  road into the chain, beside doing a thing twice to see it happen again and
  putting the wrong thing where a part goes — and the cheapest, a turn and no
  materials, which is why it pays off least often. Only a genuinely unfamiliar
  thing raises a question: a length of cord is something every person here has
  handled a thousand times, whatever else happens to be lashed together with
  it. This is what finally carries the discovery chain to its end — metal
  tools now exist in a settlement, which in every measurement before this they
  did not
- ✅ A throw parts you from the spear. Half the throws that miss put the shaft
  on the ground somewhere out past where the hunter was standing, and it is a
  spear again as soon as somebody walks over and picks it up — which is what
  makes a missed throw cost more than the walking. And what is in the hand
  when something comes at you turns some of the blow: nobody decides to get a
  spear between himself and a wolf, so the matrix carries that as a verb the
  world performs rather than one anybody chooses. It is why carrying a spear
  is worth something to a man who never hunts
- ✅ Things lie on the ground. A thing used to be either in somebody's pack or
  nowhere: an axe existed for exactly as long as its owner did, and a people
  that spent a season making them had nothing to show for it the morning after
  the man who made them drowned. A pack now falls where its owner does, and it
  is the same worn axe when the next person picks it up. Anything worth having
  within a dozen paces is worth stooping for. Food left lying goes into the
  ground in a few weeks; everything else weathers away in a season and a half
- ✅ Handing things over. A trade wants an abundance on both sides, each of
  which the other is short of; a gift wants only one, and costs the giver, and
  is worth more to the bond because it leaves somebody owing. What either of
  them counts as wanting is not a preference anybody wrote down — it is the
  raw stuff every step and every working in the chain asks for, minus what is
  already in the pack. Over eight worlds a settlement gives 328 times and
  barters once or twice: a people that gives freely has little left to bargain
  over, which is about what a band of forty who all know each other should
  look like
- ✅ Working a thing down into another thing, which is the other half of what
  a tool is for. A making puts several things together; a working takes one
  and reduces it. A core smashed into flakes — half the stone for the same
  edge — a hide cut into leather, a stick scraped into shavings. Each wants
  something in the hand and is refused without it, and the edge that did it is
  the worse for it. Shavings are a discovery: everybody knows a fire wants
  wood, and that a fire wants wood cut small enough to catch is a thing
  somebody works out with a scraper in his hand and a hearth that will not
  light. A hearth laid with tinder takes half the timber
- ✅ Every action defined by three things: what it targets, what it wants in
  the hand, and what it changes. Sixty-eight verbs across twelve families live
  in one table — `src/environment/verbs.rs` — and the table is what the
  executor consults before an action runs, so a hunt without a spear and a
  stitch with no hand free are refused in one place rather than in thirty. The
  table is honest about itself: a verb nothing performs yet says so, and a test
  fails if it stops saying so. Fifty-three of the seventy are live — the
  sixty-ninth is `freeze`, which was not in the original twelve families and
  had to be added when the fight-or-flight decision was given the rest of its
  tree
- ✅ Four ways into farming, none of which is anybody's idea about agriculture.
  Grain carried through the wet — a marsh, a riverbank, a downpour on open
  ground — starts coming up in the pack, and what falls out of a pack takes
  root where somebody was standing. Somebody who walks half a morning to the
  same berry bush lifts a slip of it and puts it in beside the tents, because
  the walk is the thing he minds. The midden the people void in comes up in
  what they ate. And a crop carried home off broken ground settles it. Whoever
  is standing near enough to see any of that happen takes the lesson
- ✅ Some plants are things nobody has tried. Four sorts grow in a world and
  which of them are supper is drawn when the country is made and written
  nowhere anybody living in it can read. A curious agent with nothing pressing
  on him occasionally eats one: if it feeds him, the people have a new food
  and can gather it; if it does not, it costs him between a bad afternoon and
  his life. Everybody standing round him learns it for nothing, which is most
  of what being a people rather than a person is worth
- ✅ Putting the wrong thing where a part goes. A man who can haft a flake to a
  stick knows the shape of the job — a shaft, a head, something to bind them —
  and can put something unexpected where the head goes. Almost always he ends
  up with a lump tied to a stick and has wasted a good stick; the materials go
  either way. Occasionally he ends up with a metal axe, and knows how to do it
  again on purpose
- ✅ Farming is worked out, and then worked at. Nobody starts out believing that
  seed put in the ground on purpose comes back as food; breaking ground is a
  hunch an agent follows out of curiosity until something proves it. Two things
  prove it: an armful carried home off broken ground, and a walk past the
  midden a season after the people voided the pips of what they ate, to find
  the same plants standing in their own refuse. A field is not sown and
  forgotten either — weeds and vermin come on in it every tick it is growing,
  and what they leave is what the farmer gets, down to a tenth of the crop on
  ground nobody has been near. Going round a field pulling weeds and picking
  pests off it is an action with a cost, chosen because the field wants it, and
  a practised hand gets round more of it in a turn. What goes in the ground is
  what is in the pack: an agent that has only ever stripped berry bushes sows
  berries, works the field all season and finds out what a berry bush thinks of
  a plough. Grain carries three times what the ground would otherwise; a berry
  bush in rows is still a berry bush
- ✅ Learned practices: nobody tells an agent that muck does a field good. It
  tries it, watches what happens, keeps or drops the idea, and the neighbours
  who saw it take something from that too
- ✅ A trade is worth having: experience comes from doing the work rather than
  from walking past it, a level costs more the higher it goes, and a trade left
  unpractised for a year starts to go. What a practised hand is worth runs from
  half to double, and it decides what comes off a field per trip and whether a
  garment is finished or spoiled in the making — so a dedicated farmer brings
  back more than a casual one, and a dedicated tailor makes better coats and
  wastes fewer. Skill used to measure how much of the map somebody had walked
  over: everybody was a master farmer, nobody had farmed, and none of it did
  anything. See ISSUES_FOUND.md #18
- ✅ Everybody is somebody: a founder is drawn with three to five compatible
  traits out of sixty-odd, and everybody born afterwards takes after their two
  parents. Forty founders are forty different people
- 🚧 Personality reaches the drives — a trait scales how loudly a drive argues
  and moves the point at which somebody acts on it, so a lazy person needs more
  pushing before starting work and a coward starts running at a smaller wolf.
  Measured against the old action ladder it changed almost nothing; both things
  that blocked it have since been rebuilt, and whether it now tells has not
  been re-measured. See ISSUES_FOUND.md #17
- ✅ Drives that know which of them matters: they are ranked primary, secondary
  and tertiary, and among the primaries the one that would kill soonest wins —
  a thirsty man stops hunting. Each drive waits on the one before it in the
  chain, so nobody puts food by while hungry or wants finery while cold. The
  ranked drives choose the action now; before this they were consulted at the
  thirteenth of thirteen fixed priorities and 79% of everything a settlement
  did was foraging. It is 25% now, and agents build and talk to each other for
  the first time
- ✅ Fear and anger that mean something: an agent weighs what is in front of it
  against what it can do about it — its health, its build, what it is carrying,
  what it can use, its nerve, and how the last fight went — and what it can
  fight makes it angry while what it cannot makes it afraid. Somebody who has
  been beaten runs where somebody who has won stands their ground. Before this,
  fear was a reading off the hunger drive and anger was written only by a blow
  that had already landed, so in a whole settlement's life nobody ever ran from
  anything or turned on anything
- ✅ And those feelings reach the hands. A frightened agent puts ground between
  itself and the thing; an angry one strikes at what is within arm's reach, and
  closes the last pace or two, but does not cross the map looking for a fight.
  It turned out nearly all the anger in the model was a grudge against a
  *person* — of every agent ready to fight, anger at people ran thirty times
  anger at creatures — held for life and with nothing downstream of it. So a
  grudge now decides the same thing: square up to somebody you cannot stand if
  you reckon you can take them, keep well clear of them if you cannot. Nobody
  raises a hand to a child, to their own parent, or to their own children
- ✅ And what an agent feels about somebody reaches what it thinks of them. A
  settlement used to have no hostile relationship anywhere in it, ever — every
  bond saturated at close, because standing near somebody was worth up to a
  tenth of the whole scale *every tick* and nothing else could be heard over
  it. Proximity and temperament are dispositions now, with a pace and a
  ceiling: a season of never leaving somebody's side makes them a familiar
  face, a season of getting on with them makes them a friend, and anything
  past that is earned by what the two of you have done. A grudge weighs on the
  bond at eight times what keeping company is worth, a blow costs a quarter of
  the scale at once, and the relationship is renamed to match — so settlements
  now contain rivals and enemies as well as friends
- ✅ Whose word an agent takes. Where the food and water are is the one thing
  agents tell each other that changes what they then do, and it used to go
  from anybody to anybody — including somebody just named an enemy — and could
  not be wrong, because the one function that weighs whether to lie was never
  called. Agents now decide whether to believe each other, from what the two
  of them are to each other, whether that one has been right before, and what
  sort of people they both are; and a man who would rather lie names a place
  that is not there. It is found out by walking to it, and what it costs him
  depends on what he lied about — sending a starving man to an empty field is
  not the same as misdescribing a pile of rocks
- ✅ News with an age, a room and a shelf life. What a man says he saw *and
  when* both travel with the claim, so somebody who reported a patch last
  season is not called a liar when it turns out to have been picked since —
  only a man who says he walked past it this morning can be held to what is
  there now. It is said out loud rather than whispered, so everybody within
  earshot hears it and each decides for themselves whether to believe him;
  and a man thinking of lying picks ground nobody present has walked lately,
  because a crowd is both more people to fool and more people who may go and
  look. What an agent keeps in its head is bounded and sorted by what it needs,
  so a thirsty man holds on to every waterhole he has heard of and forgets
  where the flax was
- ✅ A fishery, which is the one food the land does not pay for: fish come up
  the river on the season rather than growing back out of what is left of them,
  so a reach fished out fills again from the catchment. What is left of a fish,
  put on a field, is worth forty times a unit of crop — it was grown at sea. A
  settlement that works one ends thirty thousand ticks on *better* ground than
  it started on, without its peak population moving at all
- ✅ Nutrient goes back into the ground as well as coming out of it: what a body
  passes, what spoils in a pack, what a body is when it stops, and the roots
  and stalk a plant leaves in the tile it grew in. Rot loses two fifths of what
  it works on, so the loop turns and loses on every turn — but a settlement now
  holds two thirds of its peak on ground at 0.27 fertility, where before it
  kept a quarter of it on ground at 0.058. The three that go through people are
  worth almost nothing on their own, because what goes through a person comes
  out wherever the person happens to be standing — which is exactly why carting
  muck onto a field is a thing worth learning
- ✅ Family: parents keep their children close and go to one that has strayed or
  that something is stalking; children learn skills by watching the adults
  around them, and most from their own parents
- ✅ A calendar that turns: a tick is two hours, a day twelve ticks, a season
  twenty-four days and a year 1,152 ticks. A world opens in spring, an
  eight-thousand-tick run covers seven years and all four seasons, and a life
  spans eight or nine of them. Every run before this ended on Year 0, Day 4,
  having never left the winter it began in
- 🚧 The season decides regrowth, daylight and the weather — winter snows a
  tenth of the time and no other season snows at all — but not the temperature
  a tile reports: that is frozen the first time anything asks, so a clear day
  reads the same in every season
- ✅ Drives that look past this afternoon run five times faster in an agent
  whose immediate needs are met, and a quarter as fast in one whose are not
- ✅ Drives that read the world: threat, darkness, weather, what is in the pack,
  what the ground round about is bearing, whether a child has strayed. A drive
  with nothing asking for it falls quiet instead of waiting at its ceiling
- ✅ A need presses harder the longer it goes unanswered — up to fourfold, on
  both how fast it builds and how loudly it argues — so a settlement that
  cannot feed somebody eventually loses them to it. Breeding waits on a
  surplus rather than a full stomach, children have a fraction of an adult's
  reserves against a famine, and ten days of unanswered hunger sends an agent
  out of the country it is standing in
- ✅ Agents learn what pays: every attempt is recorded against the kind of
  undertaking it was and shifts what they try next. Failures count for more
  than successes, nothing is written off before five attempts, and a hunter
  who never catches anything stops hunting
- ✅ And learn *when* it pays: every attempt goes down with what the world was
  doing at the time, so an agent can work out that a thing pays in the autumn
  and not in the spring without anybody having written down that there is such
  a thing as a season
- 🚧 Agents cannot yet hear anything: that percept channel is built but unfed
- ✅ Clothing: agents gather flax, cotton and bark, make garments and wear
  them. A garment is worth what its material is worth and what the hand that
  made it could manage; wood goes into clothes only once a fire's worth is set
  aside
- ✅ Ecology: herds are held down by the predators that live off them and by
  the ground they graze. A predator that cannot find prey starves, widens what
  it will take, and turns on the people living beside it. A species wiped out of a
  world is slowly replaced by animals wandering in from off the map
- ✅ Hunting: agents go after animals for the skins and eat what comes with
  them. A hunter has to be within a spear's throw, an unarmed one leaves the
  dangerous animals alone, and a kill is butchered into meat, hides, leather
  and wool

Legend: ✅ Implemented and running | 🚧 Built but not fully connected | 📋 Not yet driven

## Project Structure

```
ebss-project/
├── src/
│   ├── core/           # Behavior trees, drives, learning algorithms
│   ├── agents/         # Agent state, lifecycle, decision-making
│   ├── environment/    # Environment abstraction and plugins
│   ├── world/          # Spatial simulation, resources, physics
│   └── analytics/      # Data logging, visualization, emergence detection
├── tests/              # Unit and integration tests
├── docs/               # Documentation and design documents
├── examples/           # Example simulations and tutorials
└── config/             # Environment configurations and presets
```

## Getting Started

### Prerequisites

- Rust 1.70+ (for core engine)
- Cargo (Rust package manager)
- (Optional) Lua 5.4+ (for environment plugins)

### Installation

```bash
# Clone the repository
git clone https://github.com/yourusername/ebss-project.git
cd ebss-project

# Build the project
cargo build --release

# Run tests
cargo test

# Run example simulation
cargo run --example basic_survival
```

### Quick Start

```rust
use ebss::prelude::*;

fn main() {
    // Create a simple world
    let world = World::new(GridConfig {
        size: (100, 100, 10),
        chunk_size: 16,
    });

    // Add agents with basic drives
    let mut population = Population::new();
    for _ in 0..10 {
        population.spawn_agent(AgentConfig::default());
    }

    // Run simulation
    let mut sim = Simulation::new(world, population);
    sim.run_for_ticks(1000);

    // Analyze results
    println!("Emergent behaviors: {:?}", sim.analyze_behaviors());
}
```

## Development Roadmap

All four originally planned phases are implemented. Boxes below reflect what
the code actually does, verified by running it — not what was planned.

### Phase 1: Core Foundation ✅
- [x] Project structure and build system
- [x] Behavior tree implementation with weight-based learning and pruning
- [x] Core drive system (all 15 drives)
- [x] Grid-based world with terrain, resources and regeneration
- [x] Agent actions and learning
- [x] ASCII visualization

### Phase 2: Environment Abstraction ✅
- [x] Plugin architecture (`src/environment/plugin.rs`, registry)
- [x] Material property system
- [x] Template-based crafting, smelting and clothing recipes
- [x] Minecraft-style environment (`src/environment/minecraft_survival.rs`)
- [x] Tool effectiveness calculations
- [x] Bundled `plugins/minecraft_survival` crate, a worked example of the
      plugin interface — though it duplicates the in-tree module above

### Phase 3: Social Systems ✅
- [x] Reproduction, pregnancy, birth and nursing
- [x] Genetic and behavioral inheritance
- [x] Observational learning
- [x] Social memory, relationships, gossip and shared knowledge
- [x] All 15 drives implemented and acted on

### Phase 4: Analytics and Polish 🚧
- [x] Data logging and analysis (metrics, export to JSON/CSV)
- [x] Emergence detection
- [x] Save/load and autosave with checkpoint rotation
- [x] Interactive GUI (egui) alongside the ASCII renderer
- [ ] Analytics are not driven by the simulation loop — they run only when a
      caller feeds them, as `examples/ascii_simulation.rs` does
- [ ] Web-based visualization: an HTTP API exists in `analytics/web_api.rs`
      but has no call sites and no front end
- [x] Bevy front end (`cargo run --features bevy_gui --bin ebss_bevy`)
- [ ] Performance has not been profiled at scale

### Beyond the original plan
- [ ] Stop a people gathering to the limit of what is in front of them rather
      than the limit of what they will eat: correcting resource clustering
      doubled what a world holds and doubled what rots rather than what is
      eaten
- [ ] Let an agent in a corner do something other than refuse to run: three
      impassable directions and `FleeFrom` fails forever, which in one measured
      world was three quarters of every turn taken
- [ ] Stop stacking food onto food and keeping the wrong clock: a dried strip
      buried onto a raw stack of the same name inherits the raw one's freshness
- [ ] Give world generation a seed, so runs are reproducible and six flaky
      tests become deterministic
- [ ] Feed the remaining percept channels: agents discover the world by sight
      now, but still cannot see each other or hear anything
- [ ] Give agents a way to make and keep tools, stores and anything decorative:
      three drives now ask for those and nothing in the world answers them
- [ ] Give the ground a way back: every measure against overshoot so far works
      by holding the population down, and none touches the constraint that
      growing food permanently lowers the rate at which more of it arrives
- [ ] Characterise long-run behaviour past 100k ticks

## Core Concepts

### Behavior Trees
Agents maintain forests of behavior trees where successful patterns are reinforced over time. Each tree branch has a weight that increases with positive outcomes.

### Drive System
15 core drives motivate agent behavior, in the order they appear in
`DriveType`:
1. Hunger - Seek and consume food
2. Thirst - Find and drink water
3. Rest - Sleep and recover from fatigue
4. Shelter - Build or locate protective structures
5. Safety - Avoid threats, create defenses
6. Preparedness - Stockpile resources and tools
7. Industry - Mine, smelt, and process materials
8. Sustenance - Farm and produce food
9. Curiosity - Explore and learn
10. Social - Interact with other agents
11. Reproduction - Create offspring
12. Luxury - Seek rare or decorative items
13. Utility - Maintain tools and equipment
14. Construction - Build structures and infrastructure
15. Protection - Keep one's children close and safe

The design document specifies thirteen (Thirst and Protection came later) and
gives each drive a list of conditions that should raise it. Nine of the fifteen
read those conditions — threat, darkness, weather, what is in the pack, what the
ground round about is bearing, whether a child has strayed — and settle where
the situation puts them; the other six build with time, which is what the
document says of them. See **The drive system against its specification** in
[SIMULATION_AUDIT.md](SIMULATION_AUDIT.md) for what that changed and for the
three drives that stay high because nothing in the world can answer them.

They are not equal, and they are not independent. Each has a rank:

- **Primary** (Hunger, Sustenance, Thirst, Rest, Safety) — immediate survival.
  Among these the one that would kill this agent soonest wins, worked out from
  what it actually has left rather than from a table, so a hungry man who is
  also dying of thirst goes to the water.
- **Secondary** (Curiosity, Social, Reproduction, Shelter, Preparedness) —
  longer-term survival and mental wellbeing.
- **Tertiary** (Luxury, Utility, Construction, Industry, Protection) — comfort
  and standing.

And each waits on the one before it. Hunger and Sustenance must be reliably
answered before Preparedness builds, and Preparedness before Luxury; Thirst
before Preparedness; Rest and Safety before Shelter; Safety and Reproduction
before Protection; Social before Construction and Industry, and those before
Utility; all four primaries before Reproduction. A hungry agent does not think
about putting food by, and a drive that is shut out falls quiet at the pace it
would have grown rather than waiting at its ceiling.

### Emotion
Fear and anger are appraisals rather than timers. Twice over — for a thing that
*threatens* an agent's ability to satisfy its drives, and for one that has been
*preventing* it — the agent asks whether it could fight the thing. Where it
could, the feeling is anger and it stands; where it could not, the feeling is
fear and it goes. What it reckons itself worth in a fight is its health, its
build, what armour and weapon it carries, its combat skill and its nerve,
scaled by what past fights taught it: an agent that has won before is worth
half again, one beaten every time is worth two fifths less. So two agents of
identical build appraise the same wolf differently, and the beaten one runs
where the winner stands.

Both feelings reach the hands. Fear moves an agent away from what it is afraid
of, far enough not to arrive back inside the range it started worrying at.
Anger strikes at what is within arm's reach — `Action::Fight` for a creature,
`Action::Attack` for a person — and closes the last pace or two, but no
further: the appraisal already scales a thing by how near it is, so what
angers an agent past the threshold is close by anyway.

Grudges are the same appraisal applied to people. The grudge itself is what an
agent feels; whether it comes out as squaring up or keeping clear is decided
by whether the agent reckons it can take them, re-asked every time they are in
the same place. It is read per person rather than off total anger, because
three mild grudges add up to a man who reads as ready to fight and has nobody
in particular to fight.

### Relationships
A bond runs from −1.0 to 1.0 and a name follows it: Enemy below −0.6, Rival
below −0.2, Friend above 0.5, Acquaintance between. Family is never renamed —
a brother you cannot stand is a brother.

What moves it is divided between what people *are* and what they have *done*.
Being about the same place as somebody, and finding their temperament suits
yours, are dispositions: they work slowly, over seasons, and they stop — the
first at a familiar face, the second at a friend. What takes two people past
that is what has actually happened between them: meals shared, help offered,
gifts given, children raised — and, in the other direction, a grudge nursed
or a blow struck.

Before this, proximity alone added up to a tenth of the whole scale every
tick. Everybody in a settlement loved everybody, and it was arithmetic rather
than affection.

### Trust
Trust is not a fourth book. `how_far_i_trust` answers it from what already
exists: the bond, weighted heaviest, because you believe your friends; whether
that person has been right before, which moves only on something the agent went
and checked; what sort of person is doing the listening — a Paranoid one
believes nobody, a Trusting one believes anybody; and what sort is doing the
talking, because a charmer gets the benefit of the doubt and somebody who puts
you off does not.

An agent below the line hears a claim and does not go and stand on it. A lie is
a real place-name moved a good walk from the real thing, and it is found out
the moment the agent looks at the spot and sees bare ground — which is also
what tells hearsay from what an agent saw for itself. Agents pass on only what
they have been to and looked at, so the man who invented a place is the man
blamed for it rather than everyone who repeated it in good faith.

A claim also carries its age. "I saw berries there" and "I saw berries there
this morning" are different claims, and only the second can be held against
the speaker when somebody finds the patch bare — a place changes, and a man
who told the truth about last season told the truth. Being out of date costs a
sixth of what a lie costs and no anger at all.

Talking is public. A speaker reaches everybody within earshot at once, and a
man weighing a lie weighs the whole room: the ground he names has to be ground
nobody standing there has walked lately, or he is contradicted before he
finishes, and every extra pair of ears is another person who may go and look.

And nothing is kept for ever. An agent holds about ninety-six places in mind,
and what it lets go of first is what answers no need it has — hearsay before
what it saw itself, and older before newer where it wants them equally.

### Memory
Agents remember:
- Spatial locations (resources, structures, landmarks)
- Storage contents with decay over time
- Social relationships and observed behaviors
- Discovered crafting recipes

### The turning year

Wild food is seasonal and so are the animals that eat it. What comes off a
carcass depends on when it was killed: a deer at the end of the autumn carries
a quarter more than the book says, the same deer at the end of the winter a
third less. The curve runs continuously round the year and is not straight —
an animal loses most of what it will lose in the first hard weeks of winter,
and puts nothing back in the first weeks of spring when there is nothing yet
to eat.

What a settlement passes goes into the ground under it, and leaves a smell and
a few seeds. Nobody lies down on fouled ground; they step off it first, which
is what puts a midden at the edge of a camp rather than in the middle of it.
The smell goes long before the matter does, and once it has, whatever came
through whole comes up as food nobody planted - on the ground the people have
moved on from, because a camp keeps its own midden too foul to grow anything
while the camp is there.

Hunting and fishing are slow work. A throw lands about a fifth of the time for
a stone-age hunter and takes a third out of the animal, so a kill is three or
four throws and every one of them costs the same whether or not it lands. A
spear-fisher takes about two fish from four casts in a good run and nothing at
all from a thin one.

### Patterns

An agent joins what it did to the need that got answered, and to the ground it
was standing on when it did — `agents::patterns`. A drive that falls by a real
amount under some action writes that action down against that need, with the
place; a drive that barely moved writes nothing, because joining a drive's own
drift to whatever the agent happened to be doing is how a superstition gets
made. An action aimed at a need that does not answer it counts against the
pattern, so ground that stops working stops being worth the walk back.

Three times is a habit and a season is as long as a place stays worth walking
to. Measured over eight worlds of ten thousand ticks, a settlement works out
about thirteen of these per living agent from nothing, and four agents in five
end up knowing where the water is.

What it does not do is change how a settlement fares. Eight worlds a side at
ten thousand ticks put the population up by 16 at a standard error of 11; eight
worlds a side at twelve thousand put it *down* by 14 at a standard error of 10.
Two runs pointing opposite ways at the same size is noise, and the honest
reading is that the mechanism costs nothing and buys nothing yet. It is kept
because it is the substrate the rest of the discovery work stands on, and
because what it records is worth having whether or not it has paid off.

### Learning
- **Trial & Error**: Random exploration with reinforcement
- **Observation**: Young agents copy experienced agents
- **Inheritance**: Offspring receive pruned parent behavior trees

## Contributing

Contributions are welcome! Please read [CONTRIBUTING.md](CONTRIBUTING.md) for details on our code of conduct and the process for submitting pull requests.

### Development Setup

```bash
# Install development dependencies
cargo install cargo-watch
cargo install cargo-tarpaulin  # For code coverage

# Run tests in watch mode
cargo watch -x test

# Check code coverage
cargo tarpaulin --out Html
```

## Documentation

- [Software Design Document](EBSS_Software_Design_Document.docx) - Original architecture and specifications
- [PROJECT_STATUS.txt](PROJECT_STATUS.txt) - Measured state of the project: what builds, what runs, what does not
- [SIMULATION_AUDIT.md](SIMULATION_AUDIT.md) - Which subsystems the simulation loop actually drives
- [ISSUES_FOUND.md](ISSUES_FOUND.md) - Current known defects, with reproduction steps
- [TESTING.md](TESTING.md) - How to run and write tests
- [SETUP.md](SETUP.md) - Development environment setup
- [docs/VISUALIZATION.md](docs/VISUALIZATION.md) - Rendering and display
- API reference: `cargo doc --open` (there is no checked-in API document)
- Examples: 21 runnable programs in [examples/](examples/), starting with
  `basic_survival.rs`

## Research Applications

EBSS is designed for:
- AI benchmarking and algorithm comparison
- Multi-agent reinforcement learning research
- Evolutionary algorithm studies
- Social science simulations
- Game AI development
- Emergent behavior analysis

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Citation

If you use EBSS in your research, please cite:

```bibtex
@software{ebss2024,
  title={Emergent Behavior Society Simulator},
  author={Your Name},
  year={2024},
  url={https://github.com/yourusername/ebss-project}
}
```

## Acknowledgments

- Inspired by Dwarf Fortress's emergent complexity
- Based on behavior tree and drive system concepts from game AI research
- Built with the Rust ecosystem

## Contact

- Issues: [GitHub Issues](https://github.com/yourusername/ebss-project/issues)
- Discussions: [GitHub Discussions](https://github.com/yourusername/ebss-project/discussions)
- Email: your.email@example.com

---

**Note**: This project is in active development. APIs and features are subject to change.
