// examples/mount_demo.rs
//! Comprehensive demonstration of the transport and mount systems
//!
//! This example shows:
//! - Different mount types and their characteristics
//! - Mounting and dismounting mechanics
//! - Stamina consumption and recovery
//! - Training and bonding with mounts
//! - Speed modifiers and combat bonuses
//! - Mount vs pack animal vs cargo transport
//! - Long-distance travel simulation

use ebss::agents::transport::{Transport, TransportType, TransportSystem};

fn main() {
    println!("=== EBSS Transport and Mount System Demonstration ===\n");

    // ===== Part 1: Mount Types and Base Stats =====
    println!("--- Part 1: Mount Types and Base Stats ---");

    let mount_types = [
        TransportType::Horse,
        TransportType::Warhorse,
        TransportType::Pony,
        TransportType::RidingCamel,
        TransportType::RidingDonkey,
        TransportType::RidingMule,
        TransportType::Reindeer,
        TransportType::Elk,
    ];

    for mount_type in mount_types {
        println!("{:?}:", mount_type);
        println!("  {}", mount_type.description());
        println!("  Speed multiplier: {:.1}x", mount_type.speed_modifier());
        println!("  Max stamina: {:.0}", mount_type.max_stamina());
        println!("  Stamina consumption: {:.2}/tick", mount_type.stamina_consumption());
        println!("  Stamina recovery: {:.2}/tick", mount_type.stamina_recovery());
        println!("  Combat bonus: {:.0}%", mount_type.combat_bonus() * 100.0);
        println!("  Carrying capacity: {:.0} kg", mount_type.weight_capacity());
        println!();
    }

    // ===== Part 2: Compare with Pack Animals and Cargo =====
    println!("--- Part 2: Comparison with Pack Animals and Cargo ---");

    println!("RIDEABLE MOUNTS (for speed):");
    println!("  Horse: {:.1}x speed, {} kg capacity",
        TransportType::Horse.speed_modifier(),
        TransportType::Horse.weight_capacity());

    println!("\nPACK ANIMALS (for cargo):");
    println!("  PackHorse: {:.1}x speed, {} kg capacity",
        TransportType::PackHorse.speed_modifier(),
        TransportType::PackHorse.weight_capacity());

    println!("\nVEHICLES (for bulk transport):");
    println!("  Cart: {:.1}x speed, {} kg capacity",
        TransportType::Cart.speed_modifier(),
        TransportType::Cart.weight_capacity());
    println!("  Wagon: {:.1}x speed, {} kg capacity",
        TransportType::Wagon.speed_modifier(),
        TransportType::Wagon.weight_capacity());
    println!();

    // ===== Part 3: Creating and Mounting =====
    println!("--- Part 3: Creating and Mounting a Horse ---");

    let mut horse = Transport::new(TransportType::Horse);
    println!("Created new horse:");
    println!("  {}", horse.mount_status());
    println!();

    // Try to mount untrained horse
    println!("Attempting to mount untrained horse...");
    match horse.mount() {
        Ok(_) => {
            println!("  Successfully mounted!");
            println!("  {}", horse.mount_status());
        }
        Err(e) => println!("  Failed: {}", e),
    }
    println!();

    // ===== Part 4: Stamina System =====
    println!("--- Part 4: Stamina Consumption and Recovery ---");

    println!("Riding for 10 ticks...");
    for tick in 1..=10 {
        horse.consume_stamina(1.0);
        if tick % 3 == 0 {
            println!("  Tick {}: Stamina {:.1}/{:.0} ({:.0}%)",
                tick,
                horse.stamina.unwrap_or(0.0),
                horse.transport_type.max_stamina(),
                horse.stamina_percentage() * 100.0);
        }
    }

    if horse.is_exhausted() {
        println!("  Horse is exhausted!");
    }
    println!();

    println!("Resting for 20 ticks...");
    horse.dismount();
    for tick in 1..=20 {
        horse.recover_stamina();
        if tick % 5 == 0 {
            println!("  Tick {}: Stamina {:.1}/{:.0} ({:.0}%)",
                tick,
                horse.stamina.unwrap_or(0.0),
                horse.transport_type.max_stamina(),
                horse.stamina_percentage() * 100.0);
        }
    }
    println!();

    // ===== Part 5: Training and Bonding =====
    println!("--- Part 5: Training and Bonding System ---");

    println!("Initial state:");
    println!("  Training: {:.0}%", horse.training_level.unwrap_or(0.0) * 100.0);
    println!("  Loyalty: {:.0}%", horse.loyalty.unwrap_or(0.0) * 100.0);
    println!();

    println!("Training the horse (10 sessions)...");
    for i in 1..=10 {
        horse.train(0.05); // 5% per session
        horse.bond(0.08); // 8% bond per session
        if i % 3 == 0 {
            println!("  Session {}: Training {:.0}%, Loyalty {:.0}%",
                i,
                horse.training_level.unwrap_or(0.0) * 100.0,
                horse.loyalty.unwrap_or(0.0) * 100.0);
        }
    }

    println!("\nFinal trained state:");
    println!("  {}", horse.mount_status());
    println!("  Will flee: {}", horse.will_flee());
    println!();

    // ===== Part 6: Effective Speed Calculation =====
    println!("--- Part 6: Effective Speed with Modifiers ---");

    // Mount and ride
    horse.mount().ok();

    // Test at different stamina levels
    println!("Speed at different stamina levels:");
    let stamina_levels = [100.0, 75.0, 50.0, 25.0, 10.0];
    for stamina_level in stamina_levels {
        horse.stamina = Some(stamina_level);
        println!("  {}% stamina: {:.2}x speed",
            (stamina_level / horse.transport_type.max_stamina() * 100.0) as u32,
            horse.effective_speed());
    }

    println!();

    // ===== Part 7: Combat Effectiveness =====
    println!("--- Part 7: Mounted Combat Comparison ---");

    let combat_mounts = [
        TransportType::Horse,
        TransportType::Warhorse,
        TransportType::Pony,
        TransportType::Elk,
    ];

    for mount_type in combat_mounts {
        let mut mount = Transport::new(mount_type);
        mount.mount().ok();

        // Test untrained
        let untrained_effectiveness = mount.combat_effectiveness();

        // Test fully trained
        mount.training_level = Some(1.0);
        let trained_effectiveness = mount.combat_effectiveness();

        println!("{:?}:",mount_type);
        println!("  Untrained: {:.0}% bonus", untrained_effectiveness * 100.0);
        println!("  Fully trained: {:.0}% bonus", trained_effectiveness * 100.0);
    }
    println!();

    // ===== Part 8: TransportSystem Integration =====
    println!("--- Part 8: TransportSystem with Multiple Mounts ---");

    let mut system = TransportSystem::new();

    // Add different transports
    let horse = Transport::new(TransportType::Horse);
    let horse_id = horse.id;
    system.add_transport(horse);

    let warhorse = Transport::new(TransportType::Warhorse);
    let warhorse_id = warhorse.id;
    system.add_transport(warhorse);

    let camel = Transport::new(TransportType::RidingCamel);
    let _camel_id = camel.id;
    system.add_transport(camel);

    let cart = Transport::new(TransportType::Cart);
    let cart_id = cart.id;
    system.add_transport(cart);

    println!("Available mounts: {}", system.get_available_mounts().len());
    println!();

    // Mount the horse
    println!("Mounting the horse...");
    match system.mount_transport(&horse_id) {
        Ok(_) => {
            println!("  Success!");
            println!("  Is mounted: {}", system.is_mounted());
            println!("  Effective speed: {:.2}x", system.effective_speed_modifier());
            println!("  Combat bonus: {:.0}%", system.mounted_combat_bonus() * 100.0);
        }
        Err(e) => println!("  Failed: {}", e),
    }
    println!();

    // Try to mount another (should fail)
    println!("Trying to mount warhorse while already mounted...");
    match system.mount_transport(&warhorse_id) {
        Ok(_) => println!("  Success!"),
        Err(e) => println!("  Failed: {}", e),
    }
    println!();

    // Dismount and compare speeds
    println!("Comparing speeds:");
    println!("  On horse: {:.2}x", system.effective_speed_modifier());
    system.dismount_current();
    println!("  Dismounted: {:.2}x", system.effective_speed_modifier());

    // Activate cart
    system.activate(&cart_id);
    println!("  With cart: {:.2}x", system.effective_speed_modifier());
    println!();

    // ===== Part 9: Long Distance Travel Simulation =====
    println!("--- Part 9: Long Distance Journey (100 km) ---");

    let distance_km = 100.0;
    let base_speed_kmh = 5.0; // Walking speed

    // Scenario 1: On foot
    let walking_time = distance_km / base_speed_kmh;
    println!("Walking:");
    println!("  Distance: {} km", distance_km);
    println!("  Speed: {:.1} km/h", base_speed_kmh);
    println!("  Time: {:.1} hours", walking_time);
    println!();

    // Scenario 2: On horse
    system.mount_transport(&horse_id).ok();
    if let Some(mount) = system.get_mounted_mut() {
        mount.training_level = Some(0.5); // Half trained
        mount.stamina = Some(mount.transport_type.max_stamina());
    }

    let horse_speed = base_speed_kmh * system.effective_speed_modifier();
    let horse_time = distance_km / horse_speed;

    println!("On trained horse:");
    println!("  Distance: {} km", distance_km);
    println!("  Speed: {:.1} km/h", horse_speed);
    println!("  Time: {:.1} hours", horse_time);
    println!("  Time saved: {:.1} hours", walking_time - horse_time);
    println!();

    // Scenario 3: With cart
    system.dismount_current();
    let cart_speed = base_speed_kmh * system.effective_speed_modifier();
    let cart_time = distance_km / cart_speed;

    println!("With cart (cargo):");
    println!("  Distance: {} km", distance_km);
    println!("  Speed: {:.1} km/h", cart_speed);
    println!("  Time: {:.1} hours", cart_time);
    println!("  Time penalty: +{:.1} hours vs walking", cart_time - walking_time);
    println!();

    // ===== Part 10: Training Progression =====
    println!("--- Part 10: Training Progression Over Time ---");

    let mut training_horse = Transport::new(TransportType::Horse);
    training_horse.mount().ok();

    println!("Training schedule (speed improvement):");
    for week in 0..=10 {
        let training_level = (week as f32 * 0.1).min(1.0);
        training_horse.training_level = Some(training_level);
        training_horse.stamina = Some(training_horse.transport_type.max_stamina());

        let speed = training_horse.effective_speed();
        let journey_time = distance_km / (base_speed_kmh * speed);

        println!("  Week {}: Training {:.0}%, Speed {:.2}x, 100km in {:.1}h",
            week,
            training_level * 100.0,
            speed,
            journey_time);
    }
    println!();

    // ===== Part 11: Mount Summary =====
    println!("--- Part 11: All Mounts Summary ---");

    for summary in system.mount_summary() {
        println!("  {}", summary);
    }
    println!();

    // ===== Part 12: Stamina Management During Journey =====
    println!("--- Part 12: Stamina Management on Long Journey ---");

    let mut journey_horse = Transport::new(TransportType::Horse);
    journey_horse.training_level = Some(0.8);
    journey_horse.loyalty = Some(0.9);
    journey_horse.mount().ok();

    println!("Starting journey:");
    println!("  {}", journey_horse.mount_status());
    println!();

    println!("Hour-by-hour progression:");
    for hour in 0..12 {
        // Ride for an hour
        for _ in 0..10 {
            journey_horse.consume_stamina(1.0);
        }

        let speed = journey_horse.effective_speed();
        let distance_this_hour = base_speed_kmh * speed;

        if hour % 2 == 0 {
            println!("  Hour {}: Stamina {:.0}%, Speed {:.2}x, Covered {:.1} km{}",
                hour,
                journey_horse.stamina_percentage() * 100.0,
                speed,
                distance_this_hour,
                if journey_horse.is_exhausted() { " [EXHAUSTED]" } else { "" });
        }

        // Rest every 3 hours
        if hour % 3 == 2 {
            println!("    → Taking a break to rest...");
            journey_horse.dismount();
            for _ in 0..20 {
                journey_horse.recover_stamina();
            }
            journey_horse.mount().ok();
        }
    }
    println!();

    // ===== Summary =====
    println!("=== Key Features Demonstrated ===");
    println!("✓ 8 distinct rideable mount types");
    println!("✓ Stamina system with consumption and recovery");
    println!("✓ Training system (0-100% training level)");
    println!("✓ Loyalty/bonding system");
    println!("✓ Dynamic speed modifiers based on stamina, training, and health");
    println!("✓ Mounted combat effectiveness bonuses");
    println!("✓ Mount vs pack animal vs cargo transport comparison");
    println!("✓ TransportSystem integration for managing multiple mounts");
    println!("✓ Exhaustion mechanics and flee behavior");
    println!("✓ Realistic long-distance travel simulation");

    println!("\n=== Demonstration Complete ===");
}
