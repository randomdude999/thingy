#![feature(gen_blocks)]

pub const WIDTH: usize = 5;
pub const HEIGHT: usize = 5;
pub const NEIGHBORS: i32 = 3;

/// returns bitmasks of positions with 3 and 2 neighbors
pub fn count_neighbors(board: u32) -> (u32, u32) {
    let a = (board >> 1) & 0b01111_01111_01111_01111_01111 & board; // has left neighbor
    let b = (board << 1) & 0b11110_11110_11110_11110_11110 & board; // has right neighbor
    let c = (board >> 5) & board; // has top neighbor
    let d = (board << 5) & board; // has bottom neighbor
    // magic formulas by the walrus
    let has_2_neighs = (a&b) | (c&d) | ((a|b)&(c|d));
    let has_3_neighs = ((a&b)&(c|d)) | ((a|b)&(c&d));
    // a better heuristic would be "has 2 neighbors and a spot where a 3rd one could be",
    // but that's much harder to compute
    (has_3_neighs, has_2_neighs)
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Board {
    // bitboards: whether this cell is occupied at all
    nonempty: u32,
    // if nonempty, does player 1 own this cell
    player: u32,
    // if nonempty, is this piece flipped
    flipped: u32,
}
impl Board {
    pub fn new() -> Self {
        Self { nonempty: 0, player: 0, flipped: 0, }
    }
    pub fn score_one_player(&self, player: bool) -> i32 {
        let board = if player { self.nonempty & self.player } else { self.nonempty & !self.player };
        let (has_3_neighs, has_2_neighs) = count_neighbors(board);
        (has_3_neighs.count_ones() * 1000 + has_2_neighs.count_ones()) as i32
    }
    /// score relative to player 0, i.e. player 0 winning is positive
    pub fn score(&self) -> i32 {
        let s0 = self.score_one_player(false);
        let s1 = self.score_one_player(true);
        s0 - s1
    }
    pub fn propagate(&mut self, player: bool) {
        let board = if player { self.nonempty & self.player } else { self.nonempty & !self.player };
        let to_flip = count_neighbors(board).0 & !self.flipped;
        self.flipped ^= to_flip;
    }
    pub fn moves(self, player: bool) -> impl Iterator<Item = Board> {
        gen move {
            let empty = !self.nonempty;
            for c in 0..25 {
                if empty & (1<<c) != 0 {
                    let mut b = self.clone();
                    b.nonempty |= 1<<c;
                    b.player |= (1<<c)*(player as u32);
                    b.propagate(player);
                    yield b;
                }
            }
            let swaplegal = self.nonempty & !self.flipped;
            for i in 0..25 {
                if swaplegal & (1<<i) == 0 { continue; }
                for j in i+1..25 {
                    if swaplegal & (1<<j) == 0 { continue; }
                    let mut b = self.clone();
                    // 0 if cell i is owned by player 0, 1 if owned by player 1
                    let i_player = (self.player >> i) & 1;
                    let j_player = (self.player >> j) & 1;
                    b.flipped |= 1<<i | 1<<j;
                    if i_player != j_player {
                        b.player ^= 1<<i | 1<<j;
                        b.propagate(false); b.propagate(true);
                    }
                    yield b;
                }
            }
        }.into_iter()
    }
}
type BoardHash = (Board, bool);

impl std::fmt::Display for Board {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(f, "===")?;
        for i in 0..25 {
            if i != 0 && i % WIDTH == 0 { writeln!(f)?; }
            let mut ch = '.';
            if self.nonempty >> i & 1 != 0 {
                // i hate this
                ch = b"xoXO"[(((self.flipped >> i & 1) << 1) | (self.player >> i & 1)) as usize] as char;
            }
            write!(f, "{}", ch)?;
        }
        Ok(())
    }
}

#[derive(Default)]
pub struct Solver {
    cache: ahash::AHashMap<BoardHash, (i32, Option<Board>)>,
    old_cache: ahash::AHashMap<BoardHash, (i32, Option<Board>)>,
    use_old_cache: bool,
}

impl Solver {
    pub fn minimax(&mut self, b: &Board, player: bool, depth: i32, mut alpha: i32, beta: i32) -> (i32,Option<Board>) {
        if depth == 0 {
            return (b.score() * (1 - 2 * player as i32), None);
        }
        let bhash = (b.clone(), player);
        // if we've seen this state on this iteration...
        if let Some((i,b)) = self.cache.get(&bhash) { return (*i,b.clone()); }
        let mut best_so_far = 0;
        let mut best_board = None;
        // if we have a previous best, check it first
        let prev_best = self.old_cache.get(&bhash).and_then(|v| v.1.clone());
        let the_iter = prev_best.iter().cloned().chain(b.clone().moves(player).filter(|x| Some(x) != prev_best.as_ref()));
        for board in the_iter {
            let s = -self.minimax(&board, !player, depth-1, -beta, -alpha).0;
            if best_board.is_none() || s > best_so_far {
                best_so_far = s;
                best_board = Some(board);
            }
            alpha = alpha.max(s);
            if alpha >= beta { break; }
        }
        if best_board.is_none() { best_so_far = b.score(); }
        self.cache.insert(bhash, (best_so_far, best_board.clone()));
        (best_so_far,best_board)
    }

    pub fn solve(&mut self, b: &Board, player: bool) -> Option<Board> {
        let mut res = None;
        const FULLDEPTH: i32 = 7;
        let start_depth = if self.use_old_cache { FULLDEPTH } else { 4 };
        for depth in start_depth..=FULLDEPTH {
            //println!("depth {}...", depth);
            let (_s,br) = self.minimax(&b, player, depth, -i32::MAX, i32::MAX);
            res = br;
            std::mem::swap(&mut self.cache, &mut self.old_cache);
            self.cache.clear();
        }
        self.use_old_cache = true;
        res
    }
}

fn main() {
    use rand::seq::SliceRandom;
    let mut rng = rand::thread_rng();
    let mut b = Board::new();
    let mut solver = Solver::default();
    let mut turn = 0;
    loop {
        if false && turn&1 == 1 {
            let moves: Vec<_> = b.moves(turn&1 == 1).collect();
            if moves.len() == 0 { return; }
            b = moves.choose(&mut rng).unwrap().clone();
        } else {
            let br = solver.solve(&b, turn&1 == 1);
            if br.is_none() { return; }
            b = br.unwrap();
        }
        turn += 1;
        println!("{}", b);
        println!("Score {:?}", b.score());
    }
}
