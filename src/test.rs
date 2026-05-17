use crate::{
    dice::{DICE_COUNT, DiceSet, DiceState, MoveType},
    game::{
        battle::{Action, Battle, PartyDef},
        content::Cnt,
    },
};

pub fn print_complete_statistics(dice_set: &DiceSet) {
    println!("STATISTICS FOR DICE:");
    for i in 0..5 {
        println!("  {:?}", dice_set[i]);
    }
    const TOTAL_COMBINATIONS: usize = 6_usize.pow(5);

    let mut wimp_out_count = 0;
    let mut flash_count = 0;
    let mut reroll_clause_count = 0;
    let mut total_score = 0;

    for result in dice_set.into_iter() {
        let state = DiceState::new(result, [true; DICE_COUNT]);
        let result = state.result();

        if result.move_options.is_none() {
            wimp_out_count += 1_u32;
        }
        if let Some(MoveType::Flash(f)) = result.move_type {
            flash_count += 1_u32;
            if f.match_count == 4 {
                reroll_clause_count += 1_u32;
            }
        }
        total_score += result.current_score;
    }

    println!(
        "WIMP OUT PERCENTAGE: {:.2}% ({wimp_out_count}/{TOTAL_COMBINATIONS})",
        wimp_out_count as f64 / TOTAL_COMBINATIONS as f64 * 100.0
    );

    println!(
        "FLASH PERCENTAGE: {:.2}% ({flash_count}/{TOTAL_COMBINATIONS})",
        flash_count as f64 / TOTAL_COMBINATIONS as f64 * 100.0
    );
    println!(
        "REROLL CLAUSE PERCENTAGE: {:.2}% ({reroll_clause_count}/{TOTAL_COMBINATIONS})",
        reroll_clause_count as f64 / TOTAL_COMBINATIONS as f64 * 100.0
    );

    println!(
        "AVERAGE SCORE: {:.2}",
        total_score as f64 / TOTAL_COMBINATIONS as f64
    );
}

pub fn health_damage_reduction(cnt: Cnt) {
    let mut rng = rand::rng();
    let mut turns_lasted_average = 0;

    for _ in 0..1000 {
        let mut battle = Battle::versus(&[&cnt.party["enn"]], &cnt.enemies["fleshthing"]);
        while battle.battle_result().is_none() {
            turns_lasted_average += 1;

            battle.push_action(Action::Flee);
            while battle.process_next_action().is_some() {}

            battle.run_enemy_turn(&mut rng);
            battle.finish_enemy_turn();
        }
    }

    println!(
        "AVERAGE TURNS LASTED (NO DEFENSE): {}",
        turns_lasted_average as f64 / 1000.0
    );

    turns_lasted_average = 0;

    for _ in 0..1000 {
        let mut battle = Battle::versus(&[&cnt.party["enn"]], &cnt.enemies["fleshthing"]);
        while battle.battle_result().is_none() {
            turns_lasted_average += 1;

            battle.push_action(Action::Defend);
            while battle.process_next_action().is_some() {}

            battle.run_enemy_turn(&mut rng);
            battle.finish_enemy_turn();
        }
    }

    println!(
        "AVERAGE TURNS LASTED (WITH DEFENSE): {}",
        turns_lasted_average as f64 / 1000.0
    );
}
