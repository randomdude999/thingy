#![feature(gen_blocks)]

use rand::SeedableRng;

pub const WIDTH: usize = 5;
pub const HEIGHT: usize = 5;
pub const NEIGHBORS: i32 = 3;

static mut ZOBRIST: [u64; WIDTH*HEIGHT*4] = [0; WIDTH*HEIGHT*4];
fn get_zobrist(ind: usize, el: u8) -> u64 {
    unsafe { ZOBRIST[ind*4 + (el & 3) as usize] }
}

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

#[derive(Debug,Clone)]
pub struct Board {
    // bitboards for both players
    upright_cells: [u32; 2],
    flipped_cells: [u32; 2],
    // precomputed hash
    hash: u64,
}
impl Board {
    pub fn new() -> Self {
        Self { upright_cells: [0, 0], flipped_cells: [0, 0], hash: 0, }
    }
    pub fn score_one_player(&self, player: usize) -> (i32, i32) {
        let board = self.upright_cells[player] | self.flipped_cells[player];
        let (has_3_neighs, has_2_neighs) = count_neighbors(board);
        return (has_3_neighs.count_ones() as i32, has_2_neighs.count_ones() as i32);
    }
    /// score relative to player 0, i.e. player 0 winning is positive
    pub fn score(&self) -> (i32, i32) {
        let s1 = self.score_one_player(0);
        let s2 = self.score_one_player(1);
        (s1.0 - s2.0, s1.1 - s2.1)
    }
    pub fn propagate(&mut self, player: usize) {
        let board = self.upright_cells[player] | self.flipped_cells[player];
        let to_flip = count_neighbors(board).0 & self.upright_cells[player];
        if to_flip != 0 {
            self.upright_cells[player] ^= to_flip;
            self.flipped_cells[player] ^= to_flip;
            let mut tmp = to_flip;
            while tmp != 0 {
                let pos = tmp.trailing_zeros();
                self.hash ^= get_zobrist(pos as usize, player as u8) ^ get_zobrist(pos as usize, 2 + player as u8);
                tmp ^= 1<<pos;
            }
        }
    }
    pub fn moves(self, player: usize) -> impl Iterator<Item = Board> {
        gen move {
            let empty = !(self.flipped_cells[0] | self.flipped_cells[1] | self.upright_cells[0] | self.upright_cells[1]);
            for c in 0..25 {
                if empty & (1<<c) != 0 {
                    let mut b = self.clone();
                    b.upright_cells[player] |= 1<<c;
                    b.hash ^= get_zobrist(c, player as u8);
                    b.propagate(player as usize);
                    yield b;
                }
            }
            let swaplegal = self.upright_cells[0] | self.upright_cells[1];
            for i in 0..25 {
                if swaplegal & (1<<i) == 0 { continue; }
                for j in i+1..25 {
                    if swaplegal & (1<<j) == 0 { continue; }
                    let mut b = self.clone();
                    // 0 if cell i is owned by player 0, 1 if owned by player 1
                    let i_player = (self.upright_cells[1] & (1<<i) != 0) as usize;
                    let j_player = (self.upright_cells[1] & (1<<j) != 0) as usize;
                    b.upright_cells[i_player] ^= 1<<i;
                    b.upright_cells[j_player] ^= 1<<j;
                    b.flipped_cells[i_player] |= 1<<j;
                    b.flipped_cells[j_player] |= 1<<i;
                    let el1 = i_player as u8;
                    let el2 = j_player as u8;
                    b.hash ^= get_zobrist(i, el1) ^ get_zobrist(j, el1+2) ^ get_zobrist(j, el2) ^ get_zobrist(i, el2+2);
                    b.propagate(player as usize);
                    yield b;
                }
            }
        }.into_iter()
    }
    pub fn hash(&self) -> u64 {
        // this is slow, to be used for correctness verification only
        let mut hash = 0u64;
        for i in 0..25 {
            if self.upright_cells[0] & (1<<i) != 0 { hash ^= get_zobrist(i, 0); }
            if self.upright_cells[1] & (1<<i) != 0 { hash ^= get_zobrist(i, 1); }
            if self.flipped_cells[0] & (1<<i) != 0 { hash ^= get_zobrist(i, 2); }
            if self.flipped_cells[1] & (1<<i) != 0 { hash ^= get_zobrist(i, 3); }
        }
        hash
    }
}
type BoardHash = u64;

impl std::fmt::Display for Board {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(f, "===")?;
        for i in 0..25 {
            if i != 0 && i % WIDTH == 0 { writeln!(f)?; }
            let mut ch = '.';
            if self.upright_cells[0] & (1<<i) != 0 { ch = 'x'; }
            if self.upright_cells[1] & (1<<i) != 0 { ch = 'o'; }
            if self.flipped_cells[0] & (1<<i) != 0 { ch = 'X'; }
            if self.flipped_cells[1] & (1<<i) != 0 { ch = 'O'; }
            write!(f, "{}", ch)?;
        }
        Ok(())
    }
}

#[derive(Default)]
pub struct Solver {
    cache: ahash::AHashMap<BoardHash, ((i32,i32),Option<Board>)>,
}
impl Solver {
    pub fn minimax(&mut self, b: &Board, player: usize, depth: i32) -> ((i32,i32),Option<Board>) {
        debug_assert!(b.hash() == b.hash);
        //assert!(b.score() == b.score);
        if depth >= 5 {
            return (b.score(), None);
        }
        if let Some((i,b)) = self.cache.get(&b.hash) { return (*i,b.clone()); }
        let f = b.clone().moves(player);
        //let mut child_avg = 0.0;
        let mut best_so_far = (0,0);
        let mut best_board = None;
        let mut l = 0;
        for (_idx,i) in f.enumerate() {
            l += 1;
            let (s,_b) = self.minimax(&i,player^1,depth+1);
            //child_avg += s.1;
            let better = if player == 0 {
                s > best_so_far
            } else {
                s < best_so_far
            };
            if best_board.is_none() || better {
                best_so_far = s;
                best_board = Some(i);
            }
        }
        //child_avg /= l as f32;
        //best_so_far.1 = child_avg;
        if best_board.is_none() { best_so_far = b.score(); }
        self.cache.insert(b.hash, (best_so_far, best_board.clone()));
        (best_so_far,best_board)
    }
}

fn init_zobrist() {
    use rand::RngCore;
    let mut rng = rand::rngs::StdRng::seed_from_u64(1337);
    for i in 0..WIDTH*HEIGHT*4 {
        unsafe {
            ZOBRIST[i] = rng.next_u64();
        }
    }
}

fn main() {
    use rand::seq::SliceRandom;
    let mut rng = rand::thread_rng();
    let mut b = Board::new();
    let mut solver = Solver::default();
    let mut turn = 0;
    init_zobrist();
    loop {
        solver.cache.clear();
        if false && turn&1 == 1 {
            let moves: Vec<_> = b.moves(turn&1).collect();
            if moves.len() == 0 { return; }
            b = moves.choose(&mut rng).unwrap().clone();
        } else {
            let (_s,br) = solver.minimax(&b, turn&1, 0);
            if br.is_none() { return; }
            b = br.unwrap();
        }
        turn += 1;
        println!("{}", b);
        println!("Score {:?}", b.score());
    }
}
