// src/world/economy.rs
//! Economic system with trading, supply/demand, and marketplace mechanics.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use crate::world::ItemType;

/// A trade offer posted by an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeOffer {
    pub id: Uuid,
    pub seller_id: Uuid,
    pub offering: Vec<(ItemType, u32)>,  // What they're selling
    pub requesting: Vec<(ItemType, u32)>, // What they want in return
    pub price: u32, // Price in abstract currency units
    pub created_tick: u32,
    pub expires_tick: u32,
}

impl TradeOffer {
    pub fn new(
        seller_id: Uuid,
        offering: Vec<(ItemType, u32)>,
        requesting: Vec<(ItemType, u32)>,
        price: u32,
        current_tick: u32,
        duration: u32,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            seller_id,
            offering,
            requesting,
            price,
            created_tick: current_tick,
            expires_tick: current_tick + duration,
        }
    }

    /// Check if this offer has expired
    pub fn is_expired(&self, current_tick: u32) -> bool {
        current_tick >= self.expires_tick
    }

}

/// Supply and demand tracker for marketplace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketData {
    /// Item type being tracked
    pub item: ItemType,

    /// Total supply available (items posted for sale)
    pub supply: u32,

    /// Total demand (items requested in trade offers)
    pub demand: u32,

    /// Base price for this item type
    pub base_price: u32,

    /// Current market price (adjusted by supply/demand)
    pub current_price: u32,

    /// Price history (last 10 ticks)
    pub price_history: Vec<u32>,

    /// Total volume traded (lifetime)
    pub volume_traded: u32,
}

impl MarketData {
    pub fn new(item: ItemType, base_price: u32) -> Self {
        Self {
            item,
            supply: 0,
            demand: 0,
            base_price,
            current_price: base_price,
            price_history: vec![base_price],
            volume_traded: 0,
        }
    }

    /// Update market price based on supply and demand
    pub fn update_price(&mut self) {
        // Price calculation: base_price * (demand / supply)
        // With protections against division by zero and extreme fluctuations

        let supply_f = self.supply.max(1) as f32;
        let demand_f = self.demand as f32;

        // Ratio of demand to supply (0.1 to 10.0 range)
        let ratio = (demand_f / supply_f).max(0.1).min(10.0);

        // New price = base_price * ratio
        let new_price = (self.base_price as f32 * ratio).round() as u32;

        // Smooth price changes (max 20% change per update)
        let max_change = (self.current_price as f32 * 0.2).max(1.0) as u32;
        let price_diff = if new_price > self.current_price {
            (new_price - self.current_price).min(max_change)
        } else {
            (self.current_price - new_price).min(max_change)
        };

        self.current_price = if new_price > self.current_price {
            self.current_price + price_diff
        } else {
            self.current_price.saturating_sub(price_diff)
        }.max(1); // Minimum price of 1

        // Update price history (keep last 10 entries)
        self.price_history.push(self.current_price);
        if self.price_history.len() > 10 {
            self.price_history.remove(0);
        }
    }

    /// Record a trade
    pub fn record_trade(&mut self, quantity: u32) {
        self.volume_traded += quantity;
    }


    /// Get price trend (-1 = falling, 0 = stable, 1 = rising)
    pub fn price_trend(&self) -> i8 {
        if self.price_history.len() < 2 {
            return 0;
        }

        let recent = *self.price_history.last().unwrap();
        let older = self.price_history[self.price_history.len() - 2];

        if recent > older + (self.base_price / 10) {
            1 // Rising
        } else if recent + (self.base_price / 10) < older {
            -1 // Falling
        } else {
            0 // Stable
        }
    }
}

/// Marketplace where agents can trade
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Marketplace {
    /// All active trade offers
    pub offers: Vec<TradeOffer>,

    /// Market data for each item type
    pub market_data: HashMap<ItemType, MarketData>,

    /// Completed trades (for history)
    pub completed_trades: Vec<CompletedTrade>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletedTrade {
    pub seller_id: Uuid,
    pub buyer_id: Uuid,
    pub items: Vec<(ItemType, u32)>,
    pub price: u32,
    pub tick: u32,
}

impl Marketplace {
    pub fn new() -> Self {
        let mut market_data = HashMap::new();

        // Initialize base prices for all item types
        for item_type in ItemType::all_types() {
            let base_price = Self::get_base_price(item_type);
            market_data.insert(item_type, MarketData::new(item_type, base_price));
        }

        Self {
            offers: Vec::new(),
            market_data,
            completed_trades: Vec::new(),
        }
    }

    /// Get base price for an item type
    fn get_base_price(item: ItemType) -> u32 {
        match item {
            // Basic resources - cheap
            ItemType::Wood | ItemType::Stone | ItemType::Food | ItemType::Water => 2,

            // Agricultural resources
            ItemType::Grain | ItemType::Flax | ItemType::Herbs | ItemType::Cotton => 3,
            // Thin stuff, and there for the picking most of spring
            ItemType::Greens => 1,
            ItemType::Roots => 2,

            // Animal products
            ItemType::Hides | ItemType::Wool | ItemType::Meat |
            ItemType::Milk | ItemType::Fish | ItemType::Honey => 4,

            // Minerals
            ItemType::Clay | ItemType::Sand | ItemType::Coal => 3,
            // Dear, because it comes from one or two places in a whole
            // country and it is the difference between eating in February
            // and not
            ItemType::Salt => 12,
            ItemType::Copper => 8,  // Copper age material
            ItemType::Tin => 6,     // Bronze alloy ingredient
            ItemType::Iron => 10,

            // Processed metals
            ItemType::Bronze => 25,
            ItemType::Steel => 50,

            // Processed materials
            ItemType::Flour | ItemType::Leather | ItemType::Cloth | ItemType::Linen |
            ItemType::Charcoal | ItemType::Rope | ItemType::Paper | ItemType::Dye => 6,
            ItemType::Glass | ItemType::Bricks => 8,

            // Finished food
            ItemType::Bread | ItemType::Ale | ItemType::Cheese => 8,

            // Finished goods
            ItemType::Clothing | ItemType::Shoes | ItemType::Pottery | ItemType::Furniture => 15,
            ItemType::Jewelry => 50,

            // Tools - wooden (cheap)
            ItemType::WoodenAxe | ItemType::WoodenPickaxe | ItemType::WoodenHammer => 10,

            // Tools - stone (moderate)
            ItemType::StoneAxe | ItemType::StonePickaxe | ItemType::StoneHammer => 20,

            // Tools - copper (moderate-high)
            ItemType::CopperAxe | ItemType::CopperPickaxe | ItemType::CopperHammer => 28,

            // Tools - bronze (high)
            ItemType::BronzeAxe | ItemType::BronzePickaxe | ItemType::BronzeHammer => 35,

            // Tools - iron (expensive)
            ItemType::IronAxe | ItemType::IronPickaxe | ItemType::IronHammer => 40,

            // Weapons - wooden
            ItemType::WoodenSpear | ItemType::WoodenBow => 15,
            // Weapons - stone
            ItemType::StoneSpear => 25,
            // Weapons - copper
            ItemType::CopperSpear | ItemType::CopperSword => 40,
            // Weapons - bronze
            ItemType::BronzeSpear | ItemType::BronzeSword | ItemType::BronzeBow => 55,
            // Weapons - iron
            ItemType::IronSword | ItemType::IronBow => 60,
            // Weapons - steel
            ItemType::SteelSword => 100,

            // Armor
            ItemType::LeatherArmor => 50,
            ItemType::CopperArmor => 70,
            ItemType::BronzeArmor => 90,
            ItemType::IronArmor => 100,
            ItemType::SteelArmor => 200,
        }
    }

    /// Post a new trade offer
    pub fn post_offer(&mut self, offer: TradeOffer) {
        // Update supply/demand
        for (item, quantity) in &offer.offering {
            if let Some(data) = self.market_data.get_mut(item) {
                data.supply += quantity;
            }
        }

        for (item, quantity) in &offer.requesting {
            if let Some(data) = self.market_data.get_mut(item) {
                data.demand += quantity;
            }
        }

        self.offers.push(offer);
    }

    /// Remove an offer (cancelled or completed)
    pub fn remove_offer(&mut self, offer_id: Uuid) -> Option<TradeOffer> {
        if let Some(idx) = self.offers.iter().position(|o| o.id == offer_id) {
            let offer = self.offers.remove(idx);

            // Update supply/demand
            for (item, quantity) in &offer.offering {
                if let Some(data) = self.market_data.get_mut(item) {
                    data.supply = data.supply.saturating_sub(*quantity);
                }
            }

            for (item, quantity) in &offer.requesting {
                if let Some(data) = self.market_data.get_mut(item) {
                    data.demand = data.demand.saturating_sub(*quantity);
                }
            }

            Some(offer)
        } else {
            None
        }
    }

    /// Complete a trade between buyer and seller
    pub fn complete_trade(
        &mut self,
        offer_id: Uuid,
        buyer_id: Uuid,
        current_tick: u32,
    ) -> Option<CompletedTrade> {
        if let Some(offer) = self.remove_offer(offer_id) {
            // Record completed trade
            let trade = CompletedTrade {
                seller_id: offer.seller_id,
                buyer_id,
                items: offer.offering.clone(),
                price: offer.price,
                tick: current_tick,
            };

            // Update volume traded
            for (item, quantity) in &offer.offering {
                if let Some(data) = self.market_data.get_mut(item) {
                    data.record_trade(*quantity);
                }
            }

            self.completed_trades.push(trade.clone());
            Some(trade)
        } else {
            None
        }
    }

    /// Clean up expired offers
    pub fn remove_expired_offers(&mut self, current_tick: u32) -> usize {
        let initial_count = self.offers.len();

        let expired_ids: Vec<Uuid> = self.offers
            .iter()
            .filter(|o| o.is_expired(current_tick))
            .map(|o| o.id)
            .collect();

        for id in expired_ids {
            self.remove_offer(id);
        }

        initial_count - self.offers.len()
    }



    /// Find offers selling a specific item
    pub fn find_offers_selling(&self, item: ItemType) -> Vec<&TradeOffer> {
        self.offers
            .iter()
            .filter(|o| o.offering.iter().any(|(i, _)| *i == item))
            .collect()
    }

    /// Find offers buying a specific item
    pub fn find_offers_buying(&self, item: ItemType) -> Vec<&TradeOffer> {
        self.offers
            .iter()
            .filter(|o| o.requesting.iter().any(|(i, _)| *i == item))
            .collect()
    }

}

impl Default for Marketplace {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketStatistics {
    pub total_offers: usize,
    pub total_trades: usize,
    pub active_items: usize,
}

/// Add all_types method to ItemType
impl ItemType {
    pub fn all_types() -> Vec<ItemType> {
        vec![
            // Basic resources
            ItemType::Wood, ItemType::Stone, ItemType::Iron, ItemType::Food,

            // Agricultural
            ItemType::Grain, ItemType::Flax, ItemType::Herbs, ItemType::Cotton,

            // Animal products
            ItemType::Hides, ItemType::Wool, ItemType::Meat, ItemType::Milk,
            ItemType::Fish, ItemType::Honey,

            // Minerals
            ItemType::Clay, ItemType::Sand, ItemType::Coal,

            // Processed materials
            ItemType::Flour, ItemType::Leather, ItemType::Cloth, ItemType::Linen,
            ItemType::Glass, ItemType::Bricks, ItemType::Charcoal, ItemType::Rope,
            ItemType::Paper, ItemType::Dye,

            // Finished food
            ItemType::Bread, ItemType::Ale, ItemType::Cheese,

            // Finished goods
            ItemType::Clothing, ItemType::Shoes, ItemType::Pottery,
            ItemType::Furniture, ItemType::Jewelry,

            // Tools
            ItemType::WoodenAxe, ItemType::StoneAxe, ItemType::IronAxe,
            ItemType::WoodenPickaxe, ItemType::StonePickaxe, ItemType::IronPickaxe,
            ItemType::WoodenHammer, ItemType::StoneHammer, ItemType::IronHammer,

            // Weapons
            ItemType::WoodenSpear, ItemType::WoodenBow, ItemType::StoneSpear,
            ItemType::IronSword, ItemType::IronBow, ItemType::SteelSword,

            // Armor
            ItemType::LeatherArmor, ItemType::IronArmor, ItemType::SteelArmor,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trade_offer_creation() {
        let seller = Uuid::new_v4();
        let offer = TradeOffer::new(
            seller,
            vec![(ItemType::Bread, 5)],
            vec![(ItemType::Wood, 10)],
            20,
            100,
            500,
        );

        assert_eq!(offer.seller_id, seller);
        assert_eq!(offer.offering[0].0, ItemType::Bread);
        assert_eq!(offer.price, 20);
        assert!(!offer.is_expired(200));
        assert!(offer.is_expired(600));
    }

    #[test]
    fn test_market_data_supply_demand() {
        let mut data = MarketData::new(ItemType::Bread, 10);

        data.supply = 100;
        data.demand = 50;
        let initial_price = data.current_price;
        data.update_price();

        // Supply > demand should lower price (but smoothed)
        assert!(data.current_price <= initial_price);

        data.supply = 50;
        data.demand = 100;
        let before_increase = data.current_price;
        data.update_price();

        // Demand > supply should raise price
        assert!(data.current_price >= before_increase);
    }

    #[test]
    fn test_price_smoothing() {
        let mut data = MarketData::new(ItemType::Iron, 100);

        data.supply = 10;
        data.demand = 1000;

        let initial_price = data.current_price;
        data.update_price();

        // Price should change but be smoothed (max 20% per update)
        let change = data.current_price.saturating_sub(initial_price);
        assert!(change <= (initial_price as f32 * 0.2).max(1.0) as u32 + 1);
    }

    #[test]
    fn test_marketplace_post_offer() {
        let mut market = Marketplace::new();
        let seller = Uuid::new_v4();

        let offer = TradeOffer::new(
            seller,
            vec![(ItemType::Bread, 10)],
            vec![],
            50,
            0,
            100,
        );

        market.post_offer(offer);

        assert_eq!(market.offers.len(), 1);

        // Supply should be updated
        let bread_data = market.market_data.get(&ItemType::Bread).unwrap();
        assert_eq!(bread_data.supply, 10);
    }

    #[test]
    fn test_marketplace_complete_trade() {
        let mut market = Marketplace::new();
        let seller = Uuid::new_v4();
        let buyer = Uuid::new_v4();

        let offer = TradeOffer::new(
            seller,
            vec![(ItemType::Bread, 10)],
            vec![],
            50,
            0,
            100,
        );

        let offer_id = offer.id;
        market.post_offer(offer);

        let trade = market.complete_trade(offer_id, buyer, 50);

        assert!(trade.is_some());
        assert_eq!(market.offers.len(), 0);
        assert_eq!(market.completed_trades.len(), 1);

        let completed = &market.completed_trades[0];
        assert_eq!(completed.seller_id, seller);
        assert_eq!(completed.buyer_id, buyer);
    }

    #[test]
    fn test_expired_offers_cleanup() {
        let mut market = Marketplace::new();

        let offer1 = TradeOffer::new(
            Uuid::new_v4(),
            vec![(ItemType::Bread, 5)],
            vec![],
            20,
            0,
            50,
        );

        let offer2 = TradeOffer::new(
            Uuid::new_v4(),
            vec![(ItemType::Wood, 10)],
            vec![],
            15,
            0,
            200,
        );

        market.post_offer(offer1);
        market.post_offer(offer2);

        assert_eq!(market.offers.len(), 2);

        // Clean at tick 100 - should remove first offer
        let removed = market.remove_expired_offers(100);
        assert_eq!(removed, 1);
        assert_eq!(market.offers.len(), 1);
    }

    #[test]
    fn test_price_trend() {
        let mut data = MarketData::new(ItemType::Bread, 100);

        // Rising: recent > older + threshold (120 > 100 + 10)
        data.price_history = vec![100, 120];
        assert_eq!(data.price_trend(), 1); // Rising

        // Falling: recent + threshold < older (80 + 10 < 100)
        data.price_history = vec![100, 80];
        assert_eq!(data.price_trend(), -1); // Falling

        // Stable: within threshold range
        data.price_history = vec![100, 105];
        assert_eq!(data.price_trend(), 0); // Stable
    }

    #[test]
    fn test_find_offers() {
        let mut market = Marketplace::new();

        let offer1 = TradeOffer::new(
            Uuid::new_v4(),
            vec![(ItemType::Bread, 5)],
            vec![(ItemType::Wood, 10)],
            20,
            0,
            100,
        );

        let offer2 = TradeOffer::new(
            Uuid::new_v4(),
            vec![(ItemType::Wood, 15)],
            vec![(ItemType::Bread, 3)],
            25,
            0,
            100,
        );

        market.post_offer(offer1);
        market.post_offer(offer2);

        let bread_sellers = market.find_offers_selling(ItemType::Bread);
        assert_eq!(bread_sellers.len(), 1);

        let wood_buyers = market.find_offers_buying(ItemType::Wood);
        assert_eq!(wood_buyers.len(), 1);
    }
}
