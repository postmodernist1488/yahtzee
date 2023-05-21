use std::fmt;
use std::io::Write;
use std::time::Duration;

use keys::*;
use ncurses::*;

use rand::Rng;

const REGULAR_PAIR: i16 = 0;
const HIGLIGHT_PAIR: i16 = 1;

const HELP_LINES: [&str; 20] = ["                       Yatzee rules.", 
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

enum PlayerKind {
    Human,
    AI
}

struct Turn {
    player: PlayerKind,
    n: u32
}

impl Default for Turn {
    fn default() -> Self {
        Turn {
            player: PlayerKind::Human,
            n: 1
        }
    }
}

impl Turn {
    fn next(&mut self) {
        match self.player {
            PlayerKind::Human => self.player = PlayerKind::AI,
            PlayerKind::AI => {
                self.player = PlayerKind::Human;
                self.n += 1;
            }
        }
    }

}

impl fmt::Display for Turn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.player {
            PlayerKind::Human => {
                write!(f, "Your turn ({})", self.n)
            }
            PlayerKind::AI => {
                write!(f, "AI turn ({})", self.n)
            }
        }
    }
}

#[derive(Default, Debug)]
struct PlayerData {
    score: i32,
    combinations_scores: [u8; 13],
    combinations_used: [bool; 13],
    got_upper_bonus: bool,
}

impl PlayerData {
    fn has_used(&self, index: usize) -> bool {
        self.combinations_used[index]
    }
    fn upper_sum(&self) -> i32 {
        self.combinations_scores[0..6].iter().sum::<u8>() as i32
    }
    fn add_score(&mut self, index: usize, score: u8) {
        self.combinations_scores[index] = score;
        self.combinations_used[index] = true;
        self.score += score as i32;
        if !self.got_upper_bonus &&
            index <= Combinations::Sixes as usize && 
            self.upper_sum() >= 63 {
            self.score += 35 as i32;
            self.got_upper_bonus = true;
        }
    }
}

#[derive(Default)]
struct GameState {
    turn: Turn,
    player: PlayerData,
    ai: PlayerData
}

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
    while getch() != KEY_NEWLINE {};
}

fn randomize_dice(dice: &mut [u8], to_randomize: &Vec<u8>) {
    for i in to_randomize {
        dice[*i as usize] = rand::thread_rng().gen_range(1..=6);
    }
}

mod keys {
    pub const KEY_Q      : i32 = 'q' as i32;
    pub const KEY_H      : i32 = 'h' as i32;
    pub const KEY_L      : i32 = 'l' as i32;
    pub const KEY_J      : i32 = 'j' as i32;
    pub const KEY_K      : i32 = 'k' as i32;
    pub const KEY_NEWLINE: i32 = '\n' as i32;
}
#[allow(dead_code)]
enum Combinations {
    Aces             = 0,
    Twos             = 1,
    Threes           = 2,
    Fours            = 3,
    Fives            = 4,
    Sixes            = 5,
    ThreeOfAKind     = 6,
    FourOfAKind      = 7,
    FullHouse        = 8,
    SmallStraight    = 9,
    LargeStraight    = 10,
    Yahtzee          = 11,
    Chance           = 12,
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
        scores[Combinations::ThreeOfAKind as usize] = dice.iter().sum();
    }
    if most_frequent_count >= 4 {
        scores[Combinations::FourOfAKind as usize] = dice.iter().sum();
    }
    if most_frequent_count == 3 && second_most_frequent_count == 2 {
        scores[Combinations::FullHouse as usize] = 25;
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
        scores[Combinations::SmallStraight as usize] = 30;
    }
    if straight_len >= 5 {
        scores[Combinations::LargeStraight as usize] = 40;
    }
    if most_frequent_count >= 5 {
        scores[Combinations::Yahtzee as usize] = 50;
    }
    scores[Combinations::Chance as usize] = dice.iter().sum();
    scores
}

fn print_padded_from_right(y: i32, win_width: i32, padding: i32, to_print: &str) {
    mvaddstr(y, win_width - padding - to_print.len() as i32, to_print);
}

/// UI
const DO_NOT_HIGHLIGHT: usize = usize::MAX;
fn print_combinations(win: *mut i8,
                      pos: (i32, i32), 
                      scores: &[u8],
                      current_element: usize, 
                      game_state: &GameState) {

    let mut offset = 0;
    let (_, win_width) = get_win_size(win);

    const PADDING1: i32 = 3;
    const SCORE: &str = "Player Score";
    print_padded_from_right(pos.0 - 10, win_width, PADDING1, SCORE);
    const PADDING2: i32 = PADDING1 * 2 + SCORE.len() as i32;
    const VALUE: &str = "Value";
    print_padded_from_right(pos.0 - 10, win_width, PADDING2, VALUE);

    for (i, &score) in scores.iter().enumerate() {

        let pair = if i == current_element {
            HIGLIGHT_PAIR
        }
        else {
            REGULAR_PAIR
        };

        attron(COLOR_PAIR(pair));
        mvaddstr(pos.0 - 9 + i as i32 + offset, pos.1, score_index_to_string(i));


        let to_print = if !game_state.player.has_used(i) {
            score.to_string()
        } else {
            "x".to_string()
        };

        print_padded_from_right(pos.0 - 9 + i as i32 + offset, win_width, PADDING2, &to_print);

        attroff(COLOR_PAIR(pair));

        let to_print = if game_state.player.has_used(i) {
            game_state.player.combinations_scores[i].to_string()
        } else {
            " ".to_string()
        };
        print_padded_from_right(pos.0 - 9 + i as i32 + offset, win_width, PADDING1, &to_print);


        if i == Combinations::Sixes as usize {
            offset += 3;
            mvaddstr(pos.0 - 9 + i as i32 + 1, pos.1, "Total score");
            let upper_sum = game_state.player.upper_sum();
            print_padded_from_right(pos.0 - 9 + i as i32 + 1, win_width, PADDING1, &upper_sum.to_string());

            mvaddstr(pos.0 - 9 + i as i32 + 2, pos.1, "Bonus (63 in total or more)");
            print_padded_from_right(pos.0 - 9 + i as i32 + 2, win_width, PADDING1,
                if game_state.player.got_upper_bonus { "35" } else { "0" });
        }
    }
}

/*
 * 
struct TurnState {
    dice: [u8; 5],
    chosen: [bool; 5],
    rolls_left: i32,
    current_element: usize,
    current_row: usize
}
let mut trn = TurnState::default();
*/

fn player_turn(win: *mut i8, game_state: &mut GameState) {

    let mut dice = [0u8; 5];
    randomize_dice(&mut dice, &(0..=4).collect());
    let mut chosen = [false; 5];
    let mut rolls_left = 2;
    let mut current_element: usize = 0;
    let mut current_row = 0;

    let (win_height, win_width) = get_win_size(win);
    let mut scores = calculate_scores(&dice);

    while rolls_left > 0 {
        update(game_state);

        mvaddstr(win_height / 2 - 3, 0, &format!("Rolls left: {}", rolls_left));
        print_combinations(win, (win_height / 2, win_width / 2), 
                           &scores, if current_element == 7 {current_row} else {DO_NOT_HIGHLIGHT}, game_state);

        for i in 0..5 {
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
            7 => (),
            _ => unreachable!(),

        }

        mvaddstr(win_height / 2, 21, "Reroll");
        mvaddstr(win_height / 2, 30, "Hold");

        let key = getch();
        match key {
            KEY_LEFT | KEY_H => {
                current_element = (current_element as i32 - 1).rem_euclid(8) as usize;
            }
            KEY_RIGHT | KEY_L => {
                current_element = (current_element + 1).rem_euclid(8);
            }

            KEY_UP | KEY_K => {
                match current_element {
                    0..=4 => {
                        chosen[current_element] = true;
                    }
                    7 => {
                        if current_row > 0 {
                            current_row -= 1;
                        }
                    }
                    _ => (),
                }
            }
            KEY_DOWN | KEY_J => {
                    match current_element {
                    0..=4 => {
                        chosen[current_element] = false;
                    }
                    7 => {
                        if current_row < 12 {
                            current_row += 1;
                        }
                    }
                    _ => (),
                }
            }
            KEY_NEWLINE => {
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
                        break;
                    }
                    7 => {
                        if !game_state.player.has_used(current_row) {
                            game_state.player.add_score(current_row, scores[current_row]);
                            return;
                        }
                    }
                    _ => {
                        unreachable!();
                    }
                }
            }
            KEY_Q => {
                user_quit(win, game_state);
            }
            _ => ()
        }
    }
    dice.sort();

    loop {
        update(game_state);
        for (i, die) in dice.iter().enumerate() {
            mvaddstr(win_height / 2, 1 + i as i32 * 4, die.to_string().as_ref());
        }
        mvaddstr(win_height / 2 - 11, 30, "Choose a combination");

        print_combinations(win, (win_height / 2, win_width / 2), &scores, current_row, game_state);

        let key = getch();
        match key {
            KEY_UP | KEY_K => {
                if current_row > 0 {
                    current_row -= 1;
                }

            }
            KEY_DOWN | KEY_J => {
                if current_row < 12 {
                    current_row += 1;
                }
            }
            KEY_NEWLINE => {
                if !game_state.player.has_used(current_row) {
                    game_state.player.add_score(current_row, scores[current_row]);
                    break;
                }
            }
            KEY_Q =>{
                user_quit(win, game_state);
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
        _ => panic!("Wrong score index")
    }
}

fn update(game_state: &GameState) {
    erase();
    addstr(&game_state.turn.to_string());
    addch('\n' as u32);
    addstr(&format!("Your score: {}", game_state.player.score));
    addch('\n' as u32);
    addstr(&format!("Ai score: {}", game_state.ai.score));
    addch('\n' as u32);
}

fn wait(time: Duration) {
    refresh();
    std::io::stdout().flush().unwrap();
    std::thread::sleep(time);
}
fn ai_choice(ai: &PlayerData, scores: &[u8]) -> usize {
    let combinations_left: Vec<_> = ai.combinations_used.iter()
        .enumerate()
        .filter(|x| !*x.1)
        .map(|x| x.0)
        .collect();

    *combinations_left.iter().max_by_key(|&i| scores[*i])
        .expect("AI must have at least one combination to choose")
}

fn ai_turn(win: *mut i8, game_state: &mut GameState) {
    let (win_height, win_width) = get_win_size(win);
    update(game_state);
    print_centered(win, "Ai is rolling...");
    wait(Duration::from_millis(800));

    //////
    let mut dice = [0u8; 5];
    randomize_dice(&mut dice, &(0..5).collect());
    let scores = calculate_scores(&dice);
    let choice = ai_choice(&game_state.ai, &scores);
    game_state.ai.add_score(choice, scores[choice]);
    //////

    update(&game_state);
    print_centered(win, "Ai rolled:");
    let strs: Vec<_> = dice.iter().map(|x| x.to_string()).collect();
    let joined = strs.join(", ");
    mvaddstr(win_height / 2 + 2, (win_width - joined.len() as i32) / 2, &joined);
    wait(Duration::from_millis(1000));

    let message = format!("Ai chose: {} for {} points",
                          score_index_to_string(choice),
                          scores[choice]);
    mvaddstr(win_height / 2 + 4, (win_width - message.len() as i32) / 2, &message);
    wait(Duration::from_millis(1500));
}

fn user_quit(win: *mut i8, game_state: &GameState) {
    let mut ans = false;
    let (win_height, win_width) = get_win_size(win);
    let (center_y, center_x) = (win_height / 2, win_width / 2);

    loop {
        update(game_state);
        print_centered(win, "Are you sure you want to quit?");
        //  | | | |
        mvaddstr(center_y + 2, center_x - 4, "yes");
        mvaddstr(center_y + 2, center_x + 2, "no");
        if ans {
            mvaddch(center_y + 2, center_x - 5, '[' as u32);
            mvaddch(center_y + 2, center_x - 1, ']' as u32);
        }
        else {
            mvaddch(center_y + 2, center_x + 1, '[' as u32);
            mvaddch(center_y + 2, center_x + 4, ']' as u32);
        }
        let key = getch();
        match key {
            KEY_LEFT | KEY_H | KEY_RIGHT | KEY_L => {
                ans = !ans;
            }
            KEY_NEWLINE if ans => {
                    endwin();
                    std::process::exit(0);
            }
            KEY_NEWLINE | KEY_Q => {
                break;
            }
            _ => (),
        }
    }
}

struct Highscore {
    name: String,
    score: i32
}

impl std::fmt::Display for Highscore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.name, self.score)
    }
}

use std::fs::File;
use std::io::{self, BufReader, BufRead};

fn get_highscores(path: &str) -> Result<Vec<Highscore>, io::Error> {
    if let Ok(file) = File::open(path) {
        let reader = BufReader::new(file);

        let mut highscores = Vec::new();

        for line in reader.lines() {
            match line?.split(':').collect::<Vec<_>>()[..] {
                [name, score_str] => {
                    if let Ok(score) = score_str.trim().parse() {
                            highscores.push(Highscore { name: name.to_string(), score });
                    }
                }
                _ => (),
            }
        }
        Ok(highscores)
    } else {
        Ok(Vec::new())
    }
}

fn write_highscores(path: &str, hs: &Vec<Highscore>) {
    let mut file = File::create(path)
                    .unwrap_or_else(|e| panic!("highscore file should open {}", e));
    for h in hs {
        writeln!(file, "{}", h).expect("write shouldn't fail");
    }
}

macro_rules! wait_for {
    ($key:ident) => {
        loop {
            let key = getch();
            match key {
                KEY_Q => return,
                $key => break,
                _ => (),
            }
        }
    };
}

fn endgame_and_highscores(win: *mut i8, game_state: &GameState) {
    use std::cmp::Ordering;

    let (s1, s2) = (format!("Your score: {}\n", game_state.player.score), 
                    format!("Ai score: {}", game_state.ai.score));

    let mut final_message = vec![
        "Game ended!",
        match game_state.player.score.cmp(&game_state.ai.score) {
            Ordering::Greater => {
                "Congratulations! You won!"
            }
            Ordering::Equal => {
                "It's a tie!"
            }
            Ordering::Less => {
                "You lost!"
            }
        },
        "",
        "",
        &s1,
        &s2,
        "",
    ];

    const HIGHSCORE_PATH: &str = "highscores.txt";
    let mut highscores = get_highscores(HIGHSCORE_PATH)
        //FIXME: don't panic
        .unwrap_or_else(|e| panic!("highscore file should open {}", e));

    if highscores.first().map(|x| x.score < game_state.player.score).unwrap_or(true) {
        final_message[2] = "New highscore! Press h to see highscores";
    } else {
        final_message[2] = "Press h to see highscores";
    }

    print_centered_left_align(win, &final_message);


    wait_for!(KEY_H);

    clear();

    let mut strs = Vec::new();
    let mut strings = Vec::new();
    strs.push("HIGHSCORES:");
    strs.push("");
    if highscores.is_empty() {
        strs.push("");
        strs.push("No highscores. Press Enter to add your score");
    } else {
        for h in highscores[..std::cmp::min(highscores.len(), 10)].iter() {
            strings.push(format!("{}\n", h));
        }
        for s in strings.iter() {
            strs.push(s);
        }
        strs.push("");
        strs.push("Press Enter to add your score");
    }
    print_centered_left_align(win, &strs);

    wait_for!(KEY_NEWLINE);

    clear();
    let (win_height, win_width) = get_win_size(win);
    mvaddstr(win_height / 2, win_width / 2 - 20, "Enter your name: ");
    let mut input = String::new();
    echo();
    getstr(&mut input);
    noecho();

    let h = Highscore {name: input, score: game_state.player.score};
    let pos = highscores.binary_search_by(|h1| h.score.cmp(&h1.score)).unwrap_or_else(|e| e);
    highscores.insert(pos, h);
    write_highscores(HIGHSCORE_PATH, &highscores);
    
    clear();
    print_centered_left_align(win, &["Added your score!", "Press any key to exit."]);
    getch();
    
}

//TODO: yahtzee bonus and joker rules
//TODO: save highscores
fn main() {
    let win = initscr();
    start_color();
    init_pair(REGULAR_PAIR, COLOR_WHITE, COLOR_BLACK);
    init_pair(HIGLIGHT_PAIR, COLOR_BLACK, COLOR_WHITE);
    keypad(win, true);
    curs_set(CURSOR_VISIBILITY::CURSOR_INVISIBLE);
    noecho();
    
    let prompts = ["Hello, this is Yahtzee.", "Press 'h' for help.", "Press Enter to play."];
    print_centered_left_align(win, &prompts);

    if getch() as u8 as char == 'h' {
        help(win);
    }

    let mut game_state = GameState::default();

    loop {
        match game_state.turn.player {
            PlayerKind::Human => {
                player_turn(win, &mut game_state);
            }
            PlayerKind::AI => {
                ai_turn(win, &mut game_state)
            }
        }
        game_state.turn.next();
        #[cfg(debug_assertions)] {
            game_state.turn.n = 14;
        }
        if game_state.turn.n == 14 {
            erase();
            endgame_and_highscores(win, &game_state);
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
