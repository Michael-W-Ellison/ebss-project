// src/agents/storage_integration.rs
//! Integration layer between agent inventory and world storehouse.
//!
//! Bridges the gap between the two inventory systems:
//! - Agent: BTreeMap<String, InventoryItem> (string-based IDs, weight tracking)
//! - World: BTreeMap<ItemType, Item> (enum-based, simple quantity)

use crate::world::ItemType;
use super::agent::{Inventory, InventoryItem};

/// Convert ItemType to string ID for agent inventory
pub fn item_type_to_id(item_type: ItemType) -> String {
    format!("{:?}", item_type).to_lowercase()
}

/// What a kill turns into once it is butchered.
///
/// Every species drops its own named cut - mutton, beef, deer_meat, blubber -
/// and its own kind of skin. Nothing downstream knows those names: the
/// nutrition database, the garment table and the cooking rules all speak in
/// meat, fish, hides, leather and wool. Without this a hunter came home with
/// twelve deer_meat it could neither eat nor cook.
///
/// Trophies - antlers, tusks, feathers, claws - pass through unchanged. They
/// have no use yet, and inventing one here would be worse than carrying them.
pub fn butchered_item_id(material_id: &str) -> &str {
    match material_id {
        "fish_meat" => "fish",

        // Every skin is a skin
        "fur" | "thick_hide" | "hide" | "snake_skin" | "pelt" => "hides",

        // Named cuts and the odd rendered fat
        "mutton" | "beef" | "pork" | "blubber" => "meat",
        other if other.ends_with("_meat") => "meat",

        other => other,
    }
}

/// Strip the preparation prefix from an item id.
///
/// Food that has been over a fire is carried under its own id - `cooked_fish`,
/// `burnt_meat` - because one inventory stack can hold only one preparation
/// state. Underneath it is still fish and still meat.
pub fn base_item_id(id: &str) -> &str {
    let id = id
        .strip_prefix("cooked_")
        .or_else(|| id.strip_prefix("burnt_"))
        .unwrap_or(id);

    // And a thing that has been cut up is still the thing it was cut off.
    //
    // Without this, `meatstrips` and `fishportions` resolved to no item type
    // at all, so nothing downstream would cook them, price them or put them
    // in a store - the same defect that had a kill dropping `mutton` and
    // `deer_meat` that nothing knew what to do with. See
    // `nutrition::Piece`, which reads the other half of the same name.
    id.strip_suffix("portions")
        .or_else(|| id.strip_suffix("strips"))
        .filter(|base| !base.is_empty())
        .unwrap_or(id)
}

/// Convert string ID to ItemType (best effort)
pub fn id_to_item_type(id: &str) -> Option<ItemType> {
    match base_item_id(&id.to_lowercase()) {
        // Basic Resources
        "wood" => Some(ItemType::Wood),
        "stone" => Some(ItemType::Stone),
        "iron" => Some(ItemType::Iron),
        "food" => Some(ItemType::Food),

        // Agricultural
        "grain" => Some(ItemType::Grain),
        "flax" => Some(ItemType::Flax),
        "herbs" => Some(ItemType::Herbs),
        "cotton" => Some(ItemType::Cotton),

        // Animal
        "hides" => Some(ItemType::Hides),
        "wool" => Some(ItemType::Wool),
        "meat" => Some(ItemType::Meat),
        "milk" => Some(ItemType::Milk),
        "fish" => Some(ItemType::Fish),
        "honey" => Some(ItemType::Honey),

        // Mineral
        "clay" => Some(ItemType::Clay),
        // Salt, greens and roots all existed as `ItemType`s - salt has had a
        // trade value of twelve since the economy was written - and none of
        // the three was in this table, which is the one place that turns a
        // thing in a pack into a thing the world can price or store. So an
        // agent holding salt was refused when it tried to put any by, six
        // hundred and sixty-six times a world. Third time this table has
        // drifted from the vocabulary beside it.
        "salt" => Some(ItemType::Salt),
        "greens" => Some(ItemType::Greens),
        "roots" => Some(ItemType::Roots),
        // The mast, under whatever name the tree that dropped it goes by.
        // Fifth time this table has had to be told about a vocabulary beside
        // it, and this one was named at the top of the nutrition scale
        // before anything in the world yielded it.
        "nuts" | "acorns" | "hazelnuts" | "chestnuts" | "walnuts" => Some(ItemType::Nuts),
        "legumes" | "beans" | "peas" | "lentils" | "chickpeas" | "vetch" => Some(ItemType::Legumes),

        // What the flora system drops.
        //
        // Fourth time this table has drifted from the vocabulary beside it,
        // and the largest: `PlantDrop` names sixty-two things a plant can
        // give, and this table knew four of them. So every apple, berry,
        // potato and ear of wheat the flora system produced arrived in a pack
        // as a name nothing could resolve - no nutrition, no price, no place
        // in a store, and after the edibility sweep, not food at all.
        //
        // Mapped onto the types that already exist rather than inventing new
        // ones: a pear and an apple are both a handful of ordinary food to a
        // body, and the model has no reason yet to tell them apart. What is
        // *not* here is deliberate - petals, fibre, bark, straw, seeds and
        // poison mushrooms are not supper.
        "apples" | "bananas" | "baobab_fruit" | "berries" | "cherries"
        | "coconut" | "mushrooms" | "olives" | "oranges" | "pears"
        | "pumpkin" | "tomatoes" | "rose_hips" | "nuts" | "fruit" => {
            Some(ItemType::Food)
        }
        "barley" | "corn" | "rice" | "wheat" => Some(ItemType::Grain),
        "carrots" | "onions" | "potatoes" | "tubers" | "lotus_root"
        | "ginseng_root" => Some(ItemType::Roots),
        "cabbage" | "seaweed" | "bamboo_shoots" | "leaves" => Some(ItemType::Greens),
        "sand" => Some(ItemType::Sand),
        "coal" => Some(ItemType::Coal),
        // Water is a drive, a resource and an item type, and its own name did
        // not resolve to it.
        "water" => Some(ItemType::Water),

        // The whole bronze-age tier. Eighteen of the seventy-four item types
        // did not survive a round trip through their own name - every copper,
        // bronze and steel thing, plus water - which is the fifth time this
        // table has drifted from the vocabulary beside it and the reason
        // `every_item_type_tests::a_type_survives_the_round_trip_through_its_name`
        // now exists. A tool nothing can name is a tool nothing can price,
        // store, trade or put in a pit.
        "copper" => Some(ItemType::Copper),
        "tin" => Some(ItemType::Tin),
        "bronze" => Some(ItemType::Bronze),
        "steel" => Some(ItemType::Steel),

        // Processed
        "flour" => Some(ItemType::Flour),
        "leather" => Some(ItemType::Leather),
        "cloth" => Some(ItemType::Cloth),
        "linen" => Some(ItemType::Linen),
        "glass" => Some(ItemType::Glass),
        "bricks" => Some(ItemType::Bricks),
        // What clay becomes on the way to being a pot, and what it becomes
        // when the fire has had it
        "claypot" => Some(ItemType::Clay),
        "stoneware" => Some(ItemType::Pottery),
        "charcoal" => Some(ItemType::Charcoal),
        "rope" => Some(ItemType::Rope),
        "paper" => Some(ItemType::Paper),
        "dye" => Some(ItemType::Dye),

        // Finished Food
        "bread" => Some(ItemType::Bread),
        "ale" => Some(ItemType::Ale),
        "cheese" => Some(ItemType::Cheese),

        // Finished Goods
        "clothing" => Some(ItemType::Clothing),
        "shoes" => Some(ItemType::Shoes),
        "pottery" => Some(ItemType::Pottery),
        "furniture" => Some(ItemType::Furniture),
        "jewelry" => Some(ItemType::Jewelry),

        // Tools
        "woodenaxe" | "wooden_axe" => Some(ItemType::WoodenAxe),
        "stoneaxe" | "stone_axe" => Some(ItemType::StoneAxe),
        "ironaxe" | "iron_axe" => Some(ItemType::IronAxe),
        "woodenpickaxe" | "wooden_pickaxe" => Some(ItemType::WoodenPickaxe),
        "stonepickaxe" | "stone_pickaxe" => Some(ItemType::StonePickaxe),
        "ironpickaxe" | "iron_pickaxe" => Some(ItemType::IronPickaxe),
        "woodenhammer" | "wooden_hammer" => Some(ItemType::WoodenHammer),
        "stonehammer" | "stone_hammer" => Some(ItemType::StoneHammer),
        "ironhammer" | "iron_hammer" => Some(ItemType::IronHammer),
        "copperaxe" | "copper_axe" => Some(ItemType::CopperAxe),
        "bronzeaxe" | "bronze_axe" => Some(ItemType::BronzeAxe),
        "copperpickaxe" | "copper_pickaxe" => Some(ItemType::CopperPickaxe),
        "bronzepickaxe" | "bronze_pickaxe" => Some(ItemType::BronzePickaxe),
        "copperhammer" | "copper_hammer" => Some(ItemType::CopperHammer),
        "bronzehammer" | "bronze_hammer" => Some(ItemType::BronzeHammer),

        // Weapons
        "woodenspear" | "wooden_spear" => Some(ItemType::WoodenSpear),
        "woodenbow" | "wooden_bow" => Some(ItemType::WoodenBow),
        "stonespear" | "stone_spear" => Some(ItemType::StoneSpear),
        "ironsword" | "iron_sword" => Some(ItemType::IronSword),
        "ironbow" | "iron_bow" => Some(ItemType::IronBow),
        "copperspear" | "copper_spear" => Some(ItemType::CopperSpear),
        "coppersword" | "copper_sword" => Some(ItemType::CopperSword),
        "bronzespear" | "bronze_spear" => Some(ItemType::BronzeSpear),
        "bronzesword" | "bronze_sword" => Some(ItemType::BronzeSword),
        "bronzebow" | "bronze_bow" => Some(ItemType::BronzeBow),
        "copperarmor" | "copper_armor" => Some(ItemType::CopperArmor),
        "bronzearmor" | "bronze_armor" => Some(ItemType::BronzeArmor),
        "steelsword" | "steel_sword" => Some(ItemType::SteelSword),

        // Armor
        "leatherarmor" | "leather_armor" => Some(ItemType::LeatherArmor),
        "ironarmor" | "iron_armor" => Some(ItemType::IronArmor),
        "steelarmor" | "steel_armor" => Some(ItemType::SteelArmor),

        _ => None,
    }
}

/// Get weight for an item type
pub fn item_weight(item_type: ItemType) -> f32 {
    match item_type {
        // Light items
        ItemType::Food | ItemType::Bread | ItemType::Cheese => 0.5,
        ItemType::Herbs | ItemType::Paper | ItemType::Dye => 0.2,
        ItemType::Cloth | ItemType::Linen | ItemType::Clothing => 0.3,

        // Medium items
        ItemType::Wood | ItemType::Grain | ItemType::Flax | ItemType::Cotton => 1.0,
        ItemType::Hides | ItemType::Wool | ItemType::Leather => 1.5,
        ItemType::Meat | ItemType::Fish => 0.8,

        // Heavy items
        ItemType::Stone | ItemType::Clay | ItemType::Bricks => 2.0,
        ItemType::Iron | ItemType::Coal | ItemType::Charcoal => 2.5,

        // Tools - medium-heavy
        ItemType::WoodenAxe | ItemType::WoodenPickaxe | ItemType::WoodenHammer => 1.5,
        ItemType::StoneAxe | ItemType::StonePickaxe | ItemType::StoneHammer => 2.0,
        ItemType::IronAxe | ItemType::IronPickaxe | ItemType::IronHammer => 2.5,

        // Weapons
        ItemType::WoodenSpear | ItemType::WoodenBow => 1.0,
        ItemType::StoneSpear => 1.5,
        ItemType::IronSword | ItemType::IronBow | ItemType::SteelSword => 2.0,

        // Armor - heavy
        ItemType::LeatherArmor => 3.0,
        ItemType::IronArmor => 5.0,
        ItemType::SteelArmor => 6.0,

        // Other
        _ => 1.0,
    }
}

/// Try to remove items from agent inventory
/// Returns (success, actual_amount_removed)
pub fn take_from_agent_inventory(
    inventory: &mut Inventory,
    item_type: ItemType,
    amount: u32,
) -> (bool, u32) {
    let item_id = item_type_to_id(item_type);

    if let Some(removed_item) = inventory.remove_item(&item_id, amount) {
        (true, removed_item.quantity)
    } else {
        (false, 0)
    }
}

/// Try to add items to agent inventory
/// Returns (success, actual_amount_added)
pub fn add_to_agent_inventory(
    inventory: &mut Inventory,
    item_type: ItemType,
    amount: u32,
) -> (bool, u32) {
    let item_id = item_type_to_id(item_type);
    let weight = item_weight(item_type);

    let item = InventoryItem::new_with_weight(item_id, amount, weight);

    if inventory.add_item(item) {
        (true, amount)
    } else {
        // Try to add as much as possible
        let mut added = 0;
        for _i in 1..=amount {
            let partial_item = InventoryItem::new_with_weight(
                item_type_to_id(item_type),
                1,
                weight,
            );

            if inventory.add_item(partial_item) {
                added += 1;
            } else {
                break;
            }
        }

        (added == amount, added)
    }
}

/// Count specific item type in agent inventory
pub fn count_in_agent_inventory(inventory: &Inventory, item_type: ItemType) -> u32 {
    let item_id = item_type_to_id(item_type);
    inventory.get_item(&item_id)
        .map(|item| item.quantity)
        .unwrap_or(0)
}

/// Count all food items in agent inventory
pub fn count_food_in_inventory(inventory: &Inventory) -> u32 {
    // Anything with food data on it is food.
    //
    // This was a hand-written list of seven item ids - food, bread, cheese,
    // meat, fish, honey, ale - and a forager's pack holds none of them. Greens,
    // roots, grain and herbs go into packs under their own names and carry
    // nutrition like anything else, and every one of them was invisible here:
    // a settlement with twenty-six items of food in hand counted fourteen.
    //
    // `InventoryItem::is_food` is the same question `what_food_i_can_spare`
    // asks, so the counting and the deciding now agree. Two vocabularies for
    // one idea, drifting apart - see ISSUES_FOUND.md, this is the eighth.
    // Either test will do. Food carries food data when it came off a plant or
    // an animal in a live world, and a few things are food by their name alone
    // - bread, cheese, ale - which are made rather than gathered and reach a
    // pack without ever passing through the food database.
    let by_name: Vec<String> = [
        ItemType::Food,
        ItemType::Bread,
        ItemType::Cheese,
        ItemType::Meat,
        ItemType::Fish,
        ItemType::Honey,
        ItemType::Ale,
    ]
    .iter()
    .map(|&kind| item_type_to_id(kind))
    .collect();

    inventory
        .get_all_items()
        .iter()
        .filter(|(name, item)| item.is_food() || by_name.contains(name))
        .map(|(_, item)| item.quantity)
        .sum()
}

/// Count all resource items (wood, stone, iron) in agent inventory
pub fn count_resources_in_inventory(inventory: &Inventory) -> u32 {
    let resource_types = vec![
        ItemType::Wood,
        ItemType::Stone,
        ItemType::Iron,
        ItemType::Clay,
        ItemType::Sand,
        ItemType::Coal,
    ];

    resource_types.iter()
        .map(|&item_type| count_in_agent_inventory(inventory, item_type))
        .sum()
}

/// Count all tools in agent inventory
pub fn count_tools_in_inventory(inventory: &Inventory) -> u32 {
    let tool_types = vec![
        ItemType::WoodenAxe,
        ItemType::StoneAxe,
        ItemType::IronAxe,
        ItemType::WoodenPickaxe,
        ItemType::StonePickaxe,
        ItemType::IronPickaxe,
        ItemType::WoodenHammer,
        ItemType::StoneHammer,
        ItemType::IronHammer,
    ];

    tool_types.iter()
        .map(|&item_type| count_in_agent_inventory(inventory, item_type))
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_item_type_conversion() {
        let id = item_type_to_id(ItemType::Wood);
        assert_eq!(id, "wood");

        let item_type = id_to_item_type("wood");
        assert_eq!(item_type, Some(ItemType::Wood));
    }

    #[test]
    fn test_item_type_conversion_tools() {
        assert_eq!(id_to_item_type("ironaxe"), Some(ItemType::IronAxe));
        assert_eq!(id_to_item_type("iron_axe"), Some(ItemType::IronAxe));
        assert_eq!(id_to_item_type("woodenspear"), Some(ItemType::WoodenSpear));
    }

    #[test]
    fn test_item_weight() {
        assert_eq!(item_weight(ItemType::Food), 0.5);
        assert_eq!(item_weight(ItemType::Stone), 2.0);
        assert!(item_weight(ItemType::IronArmor) > 4.0);
    }

    #[test]
    fn test_add_to_agent_inventory() {
        let mut inventory = Inventory::new(20, 100.0);

        let (success, added) = add_to_agent_inventory(&mut inventory, ItemType::Food, 10);
        assert!(success);
        assert_eq!(added, 10);

        assert_eq!(count_in_agent_inventory(&inventory, ItemType::Food), 10);
    }

    #[test]
    fn test_take_from_agent_inventory() {
        let mut inventory = Inventory::new(20, 100.0);

        // Add items first
        add_to_agent_inventory(&mut inventory, ItemType::Wood, 20);

        // Remove some
        let (success, removed) = take_from_agent_inventory(&mut inventory, ItemType::Wood, 15);
        assert!(success);
        assert_eq!(removed, 15);

        assert_eq!(count_in_agent_inventory(&inventory, ItemType::Wood), 5);
    }

    #[test]
    fn test_count_food_in_inventory() {
        let mut inventory = Inventory::new(20, 100.0);

        add_to_agent_inventory(&mut inventory, ItemType::Food, 5);
        add_to_agent_inventory(&mut inventory, ItemType::Bread, 3);
        add_to_agent_inventory(&mut inventory, ItemType::Meat, 2);

        assert_eq!(count_food_in_inventory(&inventory), 10);
    }

    #[test]
    fn test_count_resources_in_inventory() {
        let mut inventory = Inventory::new(20, 100.0);

        add_to_agent_inventory(&mut inventory, ItemType::Wood, 10);
        add_to_agent_inventory(&mut inventory, ItemType::Stone, 5);
        add_to_agent_inventory(&mut inventory, ItemType::Iron, 3);

        assert_eq!(count_resources_in_inventory(&inventory), 18);
    }

    #[test]
    fn test_count_tools_in_inventory() {
        let mut inventory = Inventory::new(20, 100.0);

        add_to_agent_inventory(&mut inventory, ItemType::WoodenAxe, 1);
        add_to_agent_inventory(&mut inventory, ItemType::StonePickaxe, 1);

        assert_eq!(count_tools_in_inventory(&inventory), 2);
    }
}
