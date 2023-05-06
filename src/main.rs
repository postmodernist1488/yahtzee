use std::fmt;
use std::io::Write;
use std::time::Duration;

use ncurses::*;

use rand::Rng;

const REGULAR_PAIR: i16 = 0;
const HIGLIGHT_PAIR: i16 = 1;

static HELP_LINES: [&str; 20] = ["                       Yatzee rules.", 
    "",
    "On each turn every player rolls 5 dice.", 
    "They can save any dice they want and reroll the dice up to 2 times.",
    "Once the dice are rolled, the player crosses out one of the 13 combinations to get points",
    "The combinations: ",
    "    First six 'upper section' combinations score total of the respective cards",
    "    For example, [1, 2, 3, 3, 3] scores 1 for Aces, 2 for Twos and 3 * 3 = 9 for Threes",
    "'Lower section' combinations:",
    "    3-of-a-Kind - 3 or more dice of the same number. The score is the total of all dice.",
    "    4-of-a-Kind - 4 or more dice of the same number. The score is the total of all dice.",
    "    Small Straight - 4 consecutive numbers. 30 points.",
    "    Large Straight - 5 consecutive numbers. 40 points.",
    "    Full House - 3 dice of one number and 2 of another. 25 points.",
    "    Chance - no requirements. The score is the total of all dice.",
    "",
    "Once the players cross out all 13 combinations,",
    "the game ends and the player with the most points wind the game",
    "",
    "                           Press Enter to play"
];

fn get_win_size(win: *mut i8) -> (i32, i32) {
    let mut x = 0; 
    let mut y = 0;
    getmaxyx(win, &mut y, &mut x);
    (y, x)
}

fn print_centered_left_align(win: *mut i8, lines: &[&str]) {
    let (win_height, win_width) = get_win_size(win);
    let (center_y, center_x) = (win_height / 2, win_width / 2);

    let begin = center_y - (lines.len() / 2) as i32;
    let offset = (lines.iter().max_by_key(|s| s.len()).unwrap().len() / 2) as i32;
    for (i, line) in lines.iter().enumerate() {
        mvaddstr(begin + i as i32, center_x - offset, line);
    }
}
fn print_centered(win: *mut i8, s: &str) {
    let (win_height, win_width) = get_win_size(win);
    mvaddstr(win_height / 2, (win_width - s.len() as i32) / 2, s);
}

fn help(win: *mut i8) {
    erase();
    print_centered_left_align(win, &HELP_LINES);
    getch();
}

enum Player {
    Human,
    AI
}

struct Turn {
    player: Player,
    n: u32
}

impl Turn {

    fn next(&mut self) {
        match self.player {
            Player::Human => self.player = Player::AI,
            Player::AI => {
                self.player = Player::Human;
                self.n += 1;
            }
        }
    }

}

impl fmt::Display for Turn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.player {
            Player::Human => {
                write!(f, "Your turn ({})", self.n)
            }
            Player::AI => {
                write!(f, "AI turn ({})", self.n)
            }
        }
    }
}

fn randomize_dice(dice: &mut [u8], to_randomize: &Vec<u8>) {
    for i in to_randomize {
        dice[*i as usize] = rand::thread_rng().gen_range(1..=6);
    }
}

#[allow(dead_code)]
mod combinations {
    pub const ACES            : usize = 0;
    pub const TWOS            : usize = 1;
    pub const THREES          : usize = 2;
    pub const FOURS           : usize = 3;
    pub const FIVES           : usize = 4;
    pub const SIXES           : usize = 5;
    pub const THREE_OF_A_KIND : usize = 6;
    pub const FOUR_OF_A_KIND  : usize = 7;
    pub const FULL_HOUSE      : usize = 8;
    pub const SMALL_STRAIGHT  : usize = 9;
    pub const LARGE_STRAIGHT  : usize = 10;
    pub const YAHTZEE         : usize = 11;
    pub const CHANCE          : usize = 12;
}

fn calculate_scores(dice: &[u8]) -> [u8; 13] {

    let mut dice: [u8; 5] = dice.try_into().unwrap();
    dice.sort();
    let mut scores = [0u8; 13];
    let mut counts = [0u8; 6];

    // Upper section
    for i in 1..=6u8 {
        counts[i as usize - 1] = dice.iter().cloned().filter(|&n| n == i).count() as u8;
        scores[i as usize - 1] = counts[i as usize - 1] * i;
    }


    let mut most_frequent_count = 0;
    let mut second_most_frequent_count = 0;

    for &count in counts[0..6].iter() {
        if count > most_frequent_count {
            second_most_frequent_count = most_frequent_count;
            most_frequent_count = count;
        }
        else if count > second_most_frequent_count {
            second_most_frequent_count = count;
        }
    }

    if most_frequent_count >= 3 {
        scores[combinations::THREE_OF_A_KIND] = dice.iter().sum();
    }
    if most_frequent_count >= 4 {
        scores[combinations::FOUR_OF_A_KIND] = dice.iter().sum();
    }
    if most_frequent_count == 3 && second_most_frequent_count == 2 {
        scores[combinations::FULL_HOUSE] = 25;
    }

    let straight_len = {
        let mut max_len = 0;
        let mut cur_len = 1;
        for i in 1..dice.len() {
            if dice[i] == dice[i - 1] + 1 {
                cur_len += 1;
            }
            else if dice[i] == dice[i - 1] {
            }
            else {
                max_len = std::cmp::max(cur_len, max_len);
                cur_len = 1;
            }
        }
        std::cmp::max(cur_len, max_len)
    };
    if straight_len >= 4 {
        scores[combinations::SMALL_STRAIGHT] = 30;
    }
    if straight_len >= 5 {
        scores[combinations::LARGE_STRAIGHT] = 40;
    }
    if most_frequent_count >= 5 {
        scores[combinations::YAHTZEE] = 50;
    }
    scores[combinations::CHANCE] = dice.iter().sum();
    scores
}

/// UI

fn print_combinations(win: *mut i8,
                      pos: (i32, i32), 
                      scores: &[u8],
                      current_element: usize, 
                      game_state: &GameState) {
    for (i, &score) in scores.iter().enumerate() {

        let pair = if i == current_element {
            HIGLIGHT_PAIR
        }
        else {
            REGULAR_PAIR
        };

        attron(COLOR_PAIR(pair));
        mvaddstr(pos.0 - 9 + i as i32, pos.1, score_index_to_string(i));

        let to_print = if !game_state.player_chosen_combinations[i] {
            score.to_string()
        } else {
            "x".to_string()
        };

        let (_, win_width) = get_win_size(win);
        mvaddstr(pos.0 - 9 + i as i32, win_width - 3 - to_print.len() as i32, &to_print);
        attroff(COLOR_PAIR(pair));
    }
}

fn player_turn(win: *mut i8, game_state: &mut GameState) {
    let mut dice = [0u8; 5];
    randomize_dice(&mut dice, &(0..=4).collect());
    let mut chosen = [false; 5];
    let mut rolls_left = 2;
    let mut current_element: usize = 0;

    let (win_height, win_width) = get_win_size(win);
    let mut scores = calculate_scores(&dice);
    while rolls_left > 0 {
        update(game_state);

        mvaddstr(win_height / 2 - 3, 0, &format!("Rolls left: {}", rolls_left));
        print_combinations(win, (win_height / 2, win_width / 2), &scores, 14, game_state);

        for i in 0..5 {
            //TOOD: print unicode dice doesn't work :(
            //mvaddstr(win_height / 2, 1 + i as i32 * 4, "⚁");
            //
            mvaddstr(win_height / 2 - chosen[i] as i32, 1 + i as i32 * 4, dice[i].to_string().as_ref());
        }

        match current_element {
            0..=4 => {
                mvaddch(win_height / 2, 0 + current_element as i32 * 4, '[' as u32);
                mvaddch(win_height / 2, 2 + current_element as i32 * 4, ']' as u32);
            }
            5 => {
                mvaddch(win_height / 2, 20, '[' as u32);
                mvaddch(win_height / 2, 27, ']' as u32);
            }
            6 => {
                mvaddch(win_height / 2, 29, '[' as u32);
                mvaddch(win_height / 2, 34, ']' as u32);
            }
            _ => unreachable!()

        }

        mvaddstr(win_height / 2, 21, "Reroll");
        mvaddstr(win_height / 2, 30, "Hold");

        let key = getch();
        match key {
            KEY_LEFT => {
                current_element = (current_element as i32 - 1).rem_euclid(7) as usize;
            }
            KEY_RIGHT => {
                current_element = (current_element + 1).rem_euclid(7);
            }

            KEY_UP => {
                if let 0..=4 = current_element {
                    chosen[current_element] = true;
                }
            }
            KEY_DOWN => {
                if let 0..=4 = current_element {
                    chosen[current_element] = false;
                }
            }
            10 => {
                match current_element { 
                    0..=4 => {
                        chosen[current_element] = !chosen[current_element];
                    }
                    // Hit me
                    5 => {
                        let to_randomize = (0..=4).filter(|i| !chosen[*i as usize]).collect();
                        randomize_dice(&mut dice, &to_randomize);
                        scores = calculate_scores(&dice);
                        rolls_left -= 1;
                    }
                    // Hold
                    6 => {
                        rolls_left = 0;
                    }
                    _ => {
                        unreachable!();
                    }
                }
            }
            _ => ()
        }
    }
    current_element = 0;
    dice.sort();
    loop {
        update(game_state);
        for (i, die) in dice.iter().enumerate() {
            mvaddstr(win_height / 2, 1 + i as i32 * 4, die.to_string().as_ref());
        }
        mvaddstr(win_height / 2 - 11, 30, "Choose a combination");

        print_combinations(win, (win_height / 2, win_width / 2), &scores, current_element, game_state);

        let key = getch();
        match key {
            KEY_UP => {
                if current_element > 0 {
                    current_element -= 1;
                }

            }
            KEY_DOWN => {
                if current_element < 12 {
                    current_element += 1;
                }
            }
            10 => {
                if !game_state.player_chosen_combinations[current_element] {
                    game_state.player_chosen_combinations[current_element] = true;
                    game_state.player_score += scores[current_element] as i32;
                    break;
                }
            }
            _ => ()
        }
    }
}

fn score_index_to_string(i: usize) -> &'static str {
    match i {
        0 => "Aces",
        1 => "Twos",
        2 => "Threes",
        3 => "Fours",
        4 => "Fives",
        5 => "Sixes",
        6 => "3 of a kind",
        7 => "4 of a kind",
        8 => "Full House",
        9 => "Small Straight",
        10 => "Large Straight",
        11 => "Yahtzee (5 of a kind)", 
        12 => "Chance",
        _ => unreachable!()
    }
}

struct GameState {
    turn: Turn,
    player_score: i32,
    player_chosen_combinations: [bool; 13],
    ai_score: i32,
    ai_chosen_combinations: [bool; 13]
}

fn update(game_state: &GameState) {
    erase();
    addstr(&game_state.turn.to_string());
    addch('\n' as u32);
    addstr(&format!("Your score: {}", game_state.player_score));
    addch('\n' as u32);
    addstr(&format!("Ai score: {}", game_state.ai_score));
}


fn wait(time: Duration) {
    refresh();
    std::io::stdout().flush().unwrap();
    std::thread::sleep(time);
}

fn ai_turn(win: *mut i8, game_state: &mut GameState) {
    update(game_state);
    print_centered(win, "Ai is rolling...");
    wait(Duration::from_millis(800));
    let mut dice = [0u8; 5];
    randomize_dice(&mut dice, &(0..=4).collect());

    update(&game_state);
    print_centered(win, "Ai rolled:");

    let (win_height, win_width) = get_win_size(win);
    let strs: Vec<_> = dice.iter().map(|x| x.to_string()).collect();
    let joined = strs.join(", ");
    mvaddstr(win_height / 2 + 2, (win_width - joined.len() as i32) / 2, &joined);
    wait(Duration::from_millis(1000));

    let scores = calculate_scores(&dice);
    let combinations_left: Vec<_> = game_state.ai_chosen_combinations.iter()
        .enumerate()
        .filter(|x| !*x.1)
        .map(|x| x.0)
        .collect();

    let choice = *combinations_left.iter().max_by_key(|&i| scores[*i]).unwrap();

    let message = format!("Ai chose: {} for {} points",
                          score_index_to_string(choice),
                          scores[choice]);
    mvaddstr(win_height / 2 + 4, (win_width - message.len() as i32) / 2, &message);
    wait(Duration::from_millis(1500));
    game_state.ai_chosen_combinations[choice] = true;
    game_state.ai_score += scores[choice] as i32;
}

//TODO: yahtzee bonus and joker rules
//TODO: lower section bonus
fn main() {
    let win = initscr();
    start_color();
    init_pair(REGULAR_PAIR, COLOR_WHITE, COLOR_BLACK);
    init_pair(HIGLIGHT_PAIR, COLOR_BLACK, COLOR_WHITE);
    keypad(win, true);
    curs_set(CURSOR_VISIBILITY::CURSOR_INVISIBLE);
    
    let prompts = ["Hello, this is Yahtzee.", "Press 'h' for help.", "Press Enter to play."];
    print_centered_left_align(win, &prompts);

    if getch() as u8 as char == 'h' {
        help(win);
    }

    let mut game_state = GameState {
        turn: Turn { player: Player::Human, n: 1},

        player_score: 0,
        player_chosen_combinations: [false; 13],

        ai_score: 0,
        ai_chosen_combinations: [false; 13],
    };

    loop {
        match game_state.turn.player {
            Player::Human => {
                player_turn(win, &mut game_state);
            }
            Player::AI => {
                ai_turn(win, &mut game_state)
            }
        }
        game_state.turn.next();
        if game_state.turn.n == 14 {
            erase();
            use std::cmp::Ordering;
            
            let message = match game_state.player_score.cmp(&game_state.ai_score) {
                Ordering::Greater => {
                    "Congratulations! You won!"
                }
                Ordering::Equal => {
                    "It's a tie!"
                }
                Ordering::Less => {
                    "You lost!"
                }
            };
            print_centered_left_align(win, &[
                "Game ended!",
                message,
                "",
                &format!("Your score: {}\n", game_state.player_score),
                &format!("Ai score: {}", game_state.ai_score),
                "",
                "Press any button to exit."
            ]);
            getch();
            break;
        }
    }

    endwin();
}

#[cfg(test)]
mod test {
    use super::*;
    
    #[test]
    fn three_of_a_kind() {
        let dice = [1, 2, 3, 3, 3];
        let scores = calculate_scores(&dice);
        assert_eq!(scores, [1u8, 2, 9, 0, 0, 0, 12, 0, 0, 0, 0, 0, 12]);
    }
    #[test]
    fn four_of_a_kind() {
        let dice = [1, 3, 3, 3, 3];
        let scores = calculate_scores(&dice);
        assert_eq!(scores, [1u8, 0, 12, 0, 0, 0, 13, 13, 0, 0, 0, 0, 13]);
    }

    #[test]
    fn fullhouse() {
        let dice = [4, 4, 3, 3, 3];
        let scores = calculate_scores(&dice);
        assert_eq!(scores, [0u8, 0, 9, 8, 0, 0, 17, 0, 25, 0, 0, 0, 17]);
    }

    #[test]
    fn yahtzee() {
        let dice = [1, 1, 1, 1, 1];
        let scores = calculate_scores(&dice);
        assert_eq!(scores, [5u8, 0, 0, 0, 0, 0, 5, 5, 0, 0, 0, 50, 5]);
    }

    #[test]
    fn small_straight() {
        let dice = [3, 2, 1, 4, 3];
        let scores = calculate_scores(&dice);
        assert_eq!(scores, [1u8, 2, 6, 4, 0, 0, 0, 0, 0, 30, 0, 0, 13]);
    }

    #[test]
    fn large_straight() {
        let dice = [3, 2, 1, 4, 5];
        let scores = calculate_scores(&dice);
        assert_eq!(scores, [1u8, 2, 3, 4, 5, 0, 0, 0, 0, 30, 40, 0, 15]);
    }
}
