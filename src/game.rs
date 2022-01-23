use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::AccountId;
use near_sdk::serde::Serialize;

#[derive(BorshDeserialize, BorshSerialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub enum GameState {
    GameCreated,
    GameInitialized,
    GameEnded,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct Game {
    pub player1: AccountId,
    pub player2: Option<AccountId>,
    pub field: [u8; 9],
    pub round: u8,
    pub whose_move: bool,
    pub game_state: GameState,
    pub winner: Option<AccountId>,
}

impl Game {
    pub fn win(&self) -> bool {
        let f = &self.field;
        (f[0] == f[1] && f[0] == f[2])
            || (f[3] == f[3] && f[3] == f[5])
            || (f[6] == f[7] && f[6] == f[8])
            || (f[0] == f[3] && f[0] == f[6])
            || (f[1] == f[4] && f[1] == f[7])
            || (f[2] == f[5] && f[2] == f[8])
            || (f[0] == f[4] && f[0] == f[8])
            || (f[2] == f[4] && f[2] == f[6])
    }

    pub fn draw(&self) -> bool {
        self.round == 9
    }
}
