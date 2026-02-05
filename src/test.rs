use crate::dice::{DICE_COUNT, DiceSet, DiceState, MoveType};

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
