use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LookupMap, Vector};
use near_sdk::env::{self};
use near_sdk::{log, near_bindgen, AccountId, Balance, BorshStorageKey, PanicOnDefault, Promise};
// use rand::{rngs::StdRng, Rng, SeedableRng};

near_sdk::setup_alloc!();

mod game;
use game::{Game, GameState};

type GameId = u64;

const ONE_NEAR: Balance = 1_000_000_000_000_000_000_000_000;

const PERCENT_FEE: u8 = 5;
const DENOMINATOR: u128 = 100 / PERCENT_FEE as u128;

const ONE_SECOND: u64 = 1_000_000_000;
const ONE_MINUTE: u64 = 60 * ONE_SECOND;

#[derive(BorshStorageKey, BorshSerialize)]
pub enum StorageKeys {
    Games,
    CompleteGames,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    games: LookupMap<GameId, Game>,
    next_game_id: GameId,
    complete_games: Vector<GameId>,
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new() -> Self {
        assert!(!env::state_exists(), "Contract already initialized");

        log!("Contract initialized");

        Self {
            games: LookupMap::new(StorageKeys::Games),
            next_game_id: 0,
            complete_games: Vector::new(StorageKeys::CompleteGames),
        }
    }

    #[payable]
    pub fn new_game(&mut self) -> GameId {
        let amount = env::attached_deposit();

        assert!(amount >= ONE_NEAR, "Deposit have to be >= than 1 NEAR");

        // let seed: [u8; 32] = random_seed().try_into().unwrap();
        // let mut seeded_rng = StdRng::from_seed(seed);

        // let mut game_id: GameId = seeded_rng.gen_range(0..u64::MAX);
        // while let Some(_) = self.games.get(&game_id) {
        //     game_id = seeded_rng.gen_range(0..u64::MAX);
        // }

        let game_id = self.next_game_id;

        let game = Game {
            player1: env::predecessor_account_id(),
            deposit: amount,
            player2: None,
            field: [9; 9],
            round: 0,
            whose_move: false,
            last_move_time: None,
            game_state: GameState::GameCreated,
            winner: None,
        };

        self.games.insert(&game_id, &game);

        log!(
            "Player {} created the game with GameId: {} and deposit: {} NEAR",
            env::predecessor_account_id(),
            game_id,
            amount,
        );

        self.next_game_id += 1;

        game_id
    }

    #[payable]
    pub fn join_game(&mut self, game_id: GameId) {
        let amount = env::attached_deposit();

        assert!(
            self.games.get(&game_id).is_some(),
            "No game with such GameId"
        );

        let mut game = self.games.get(&game_id).unwrap();

        match game.game_state {
            GameState::GameCreated => {
                assert!(
                    game.deposit <= amount,
                    "Wrong deposit. Player1's bet is {} NEAR",
                    game.deposit
                );

                if amount > game.deposit {
                    let refund = amount - game.deposit;
                    Promise::new(env::predecessor_account_id()).transfer(refund);

                    log!(
                        "Refunded {} NEAR to {}",
                        refund,
                        env::predecessor_account_id()
                    );
                }

                game.player2 = Some(env::predecessor_account_id());
                game.game_state = GameState::GameInitialized;

                self.games.insert(&game_id, &game);

                log!(
                    "Player {} joined the game {}",
                    env::predecessor_account_id(),
                    game_id
                );
            }
            _ => panic!("Game is not active"),
        }
    }

    pub fn make_move(&mut self, game_id: GameId, coordinate: usize) {
        assert!(
            self.games.get(&game_id).is_some(),
            "No game with such GameId"
        );

        let mut game = self.games.get(&game_id).unwrap();

        if let GameState::GameInitialized = game.game_state {
            let whose_move: AccountId;
            if game.whose_move {
                whose_move = game.player2.clone().unwrap();
            } else {
                whose_move = game.player1.clone();
            }

            assert_eq!(
                env::predecessor_account_id(),
                whose_move,
                "Move order disrupted"
            );

            assert!(
                coordinate < 9 && game.field[coordinate] == 9,
                "Invalid move"
            );

            if game.whose_move {
                game.field[coordinate] = 1;
            } else {
                game.field[coordinate] = 0;
            }

            game.round += 1;
            game.last_move_time = Some(env::block_timestamp());

            if game.win() {
                game.game_state = GameState::GameEnded;

                game.winner = Some(env::predecessor_account_id());

                let prize = 2 * (game.deposit - game.deposit / DENOMINATOR);
                Promise::new(env::predecessor_account_id()).transfer(prize);

                log!("Winner is {}", env::predecessor_account_id());

                self.complete_games.push(&game_id);
            } else if game.draw() {
                game.game_state = GameState::GameEnded;

                let refund = game.deposit - game.deposit / DENOMINATOR;

                Promise::new(game.player1.clone()).transfer(refund);
                Promise::new(game.player2.clone().unwrap()).transfer(refund);

                log!("Draw");

                self.complete_games.push(&game_id);
            } else {
                game.whose_move = !game.whose_move;

                log!("Next move");
            }

            self.games.insert(&game_id, &game);
        } else {
            panic!("Game is not active");
        }
    }

    pub fn get_prize(&mut self, game_id: GameId) {
        assert!(
            self.games.get(&game_id).is_some(),
            "No game with such GameId"
        );

        let mut game = self.games.get(&game_id).unwrap();

        if let GameState::GameInitialized = game.game_state {
            assert!(game.round != 0);

            if game.whose_move {
                assert!(env::predecessor_account_id() == game.player1);
            } else {
                assert!(env::predecessor_account_id() == game.player2.clone().unwrap());
            }

            let interval = env::block_timestamp() - game.last_move_time.unwrap();
            if interval > ONE_MINUTE {
                game.winner = Some(env::predecessor_account_id());
                game.game_state = GameState::GameEnded;

                let prize = 2 * (game.deposit - game.deposit / DENOMINATOR);
                Promise::new(env::predecessor_account_id()).transfer(prize);

                log!(
                    "Prize is given to {} because of move time limit",
                    env::predecessor_account_id()
                );

                log!("Winner is {}", env::predecessor_account_id());
            } else {
                panic!("Last move was made {} seconds ago", interval / ONE_SECOND);
            }

            self.games.insert(&game_id, &game);
        } else {
            panic!("Game is not active");
        }
    }

    pub fn get_game_state(&self, game_id: GameId) -> Game {
        self.games.get(&game_id).expect("No game with such GameId")
    }

    #[private]
    pub fn state_cleaner_with_id(&mut self, game_id: GameId) {
        self.games.remove(&game_id);
    }

    #[private]
    pub fn state_cleaner(&mut self) {
        let game_id = self.complete_games.pop();
        while !game_id.is_none() {
            self.games.remove(&game_id.unwrap());
        }
    }
}
