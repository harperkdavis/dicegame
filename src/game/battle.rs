pub mod enemy;
pub mod item;
pub mod party;

use std::collections::VecDeque;

pub use enemy::EnemyDef;
pub use item::ItemDef;
pub use party::PartyDef;
use rand::Rng;

pub struct PartyMember {
    info: &'static PartyDef,
    health: u32,
}

impl PartyMember {
    pub fn from_info(info: &'static PartyDef) -> Self {
        Self {
            info,
            health: info.health,
        }
    }

    pub fn info(&self) -> &'static PartyDef {
        self.info
    }

    pub fn health(&self) -> u32 {
        self.health
    }
}

pub struct Enemy {
    info: &'static EnemyDef,
    health: u32,
}

impl Enemy {
    pub fn from_info(info: &'static EnemyDef) -> Self {
        Self {
            info,
            health: info.health,
        }
    }

    pub fn info(&self) -> &'static EnemyDef {
        self.info
    }

    pub fn health(&self) -> u32 {
        self.health
    }
}

pub enum Action {
    Attack(usize),
    Defend,
    Flee,
}

pub struct Battle {
    party: Vec<PartyMember>,
    enemies: Vec<Enemy>,
    actions: VecDeque<(Action, usize)>,
    is_player_turn: bool,
}

pub struct DamageEvent {
    pub from: usize,
    pub to: usize,
    pub amount: u32,
}

impl Battle {
    pub fn versus(party: &'static PartyDef, enemy: &'static EnemyDef) -> Self {
        Self {
            party: vec![PartyMember::from_info(party)],
            enemies: vec![Enemy::from_info(enemy)],
            actions: VecDeque::new(),
            is_player_turn: true,
        }
    }

    pub fn party(&self) -> &[PartyMember] {
        &self.party
    }

    pub fn enemies(&self) -> &[Enemy] {
        &self.enemies
    }

    pub fn is_player_turn(&self) -> bool {
        self.is_player_turn
    }

    pub fn push_action(&mut self, action: Action) -> bool {
        self.actions.push_back((action, self.actions.len()));
        if self.actions.len() >= self.party.len() {
            self.is_player_turn = false;
            true
        } else {
            false
        }
    }

    pub fn pop_action(&mut self) -> Option<(Action, usize)> {
        self.actions.pop_back()
    }

    pub fn current_party_member(&self) -> Option<usize> {
        self.is_player_turn().then_some(self.actions.len())
    }

    pub fn process_next_action(&mut self) -> Option<(Action, usize)> {
        if self.is_player_turn() {
            return None;
        }
        self.actions.pop_front()
    }

    pub fn apply_damage(&mut self, index: usize, damage: u32) {
        self.enemies[index].health = self.enemies[index].health.saturating_sub(damage);
    }

    pub fn battle_result(&self) -> Option<bool> {
        if self.enemies.iter().all(|e| e.health == 0) {
            Some(true)
        } else if self.party.iter().all(|e| e.health == 0) {
            Some(false)
        } else {
            None
        }
    }

    pub fn run_enemy_turn(&mut self, rng: &mut impl Rng) -> Vec<DamageEvent> {
        let mut damage = Vec::new();

        for (from, enemy) in self.enemies.iter().enumerate() {
            if enemy.health() == 0 {
                continue;
            }
            let target = rng.random_range(0..self.party.len());
            let damage_dealt =
                (enemy.info.attack as f64 * rng.random_range(0.9..1.1)).round() as u32;

            damage.push(DamageEvent {
                from,
                to: target,
                amount: damage_dealt,
            });
            self.party[target].health = self.party[target].health.saturating_sub(damage_dealt);
        }

        damage
    }

    pub fn finish_enemy_turn(&mut self) {
        self.is_player_turn = true;
    }
}
