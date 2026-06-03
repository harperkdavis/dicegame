pub mod enemy;
pub mod health;
pub mod item;
pub mod party;
pub mod text;

use std::collections::{HashMap, VecDeque};

pub use enemy::EnemyDef;
pub use health::Health;
pub use item::ItemDef;
pub use party::PartyDef;
use rand::Rng;

use crate::Str;

pub const MAX_PARTY_SIZE: usize = 4;

pub struct PartyMember {
    info: &'static PartyDef,
    health: Health,
}

impl PartyMember {
    pub fn from_info(info: &'static PartyDef) -> Self {
        Self {
            info,
            health: Health::full(),
        }
    }

    pub fn info(&self) -> &'static PartyDef {
        self.info
    }

    pub fn health(&self) -> &Health {
        &self.health
    }

    pub fn total_health(&self) -> u32 {
        self.health.total()
    }

    pub fn is_dead(&self) -> bool {
        self.health.is_dead()
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
    party_defending: [bool; MAX_PARTY_SIZE],
    party_attacked: [u32; MAX_PARTY_SIZE],
    enemies: Vec<Enemy>,
    actions: VecDeque<(Action, usize)>,
    is_player_turn: bool,
    reward_money: u32,
    reward_items: Vec<Str>,
}

pub struct DamageEvent {
    pub from: usize,
    pub to: usize,
    pub amount: u32,
}

pub struct Rewards {
    pub money: u32,
    pub items: HashMap<Str, usize>,
}

impl Battle {
    pub fn versus(party: &[&'static PartyDef], enemy: &'static EnemyDef) -> Self {
        Self {
            party: party.iter().map(|a| PartyMember::from_info(a)).collect(),
            party_defending: [false; MAX_PARTY_SIZE],
            party_attacked: [0; MAX_PARTY_SIZE],
            enemies: vec![Enemy::from_info(enemy)],
            actions: VecDeque::new(),
            is_player_turn: true,

            reward_money: 0,
            reward_items: Vec::new(),
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
        if self.battle_result().is_some() {
            self.actions.clear();
            return None;
        }
        let (action, from) = self.actions.pop_front()?;
        if let Action::Defend = action {
            self.party_defending[from] = true
        }
        Some((action, from))
    }

    fn process_enemy_death(&mut self, index: usize, rng: &mut impl Rng) {
        let enemy_def = self.enemies[index].info();
        self.reward_money = self
            .reward_money
            .saturating_add(enemy_def.calculate_reward(rng));
        self.reward_items
            .append(&mut enemy_def.calculate_drops(rng));
    }

    pub fn apply_damage(&mut self, index: usize, damage: u32, rng: &mut impl Rng) {
        self.enemies[index].health = self.enemies[index].health.saturating_sub(damage);

        if self.enemies[index].health == 0 {
            self.process_enemy_death(index, rng);
        }
    }

    pub fn battle_result(&self) -> Option<bool> {
        if self.enemies.iter().all(|e| e.health == 0) {
            Some(true)
        } else if self.party.iter().all(|e| e.is_dead()) {
            Some(false)
        } else {
            None
        }
    }
    pub fn rewards(&self) -> Option<Rewards> {
        if self.battle_result().is_some_and(|a| a) {
            let money = self.reward_money;
            let items = self
                .reward_items
                .iter()
                .fold(HashMap::new(), |mut map, item| {
                    *map.entry(item.clone()).or_insert(0) += 1;
                    map
                });

            Some(Rewards { money, items })
        } else {
            None
        }
    }

    // decide a target for enemies
    // defending party members are twice as likely to be selected
    fn decide_enemy_target(&self, rng: &mut impl Rng) -> usize {
        let candidates = self
            .party
            .iter()
            .enumerate()
            .filter_map(|(i, p)| {
                (!p.is_dead()).then_some((
                    i,
                    2_u32.pow(10 - self.party_attacked[i])
                        * if self.party_defending[i] { 2 } else { 1 },
                ))
            })
            .collect::<Box<_>>();
        let total = candidates.iter().map(|(_, weight)| *weight).sum();
        let pick = rng.random_range(0..total);
        let mut sum = 0;
        for (i, weight) in candidates {
            sum += weight;
            if sum >= pick {
                return i;
            }
        }
        0
    }

    pub fn run_enemy_turn(&mut self, rng: &mut impl Rng) -> Vec<DamageEvent> {
        let mut damage = Vec::new();

        if self.battle_result().is_some() {
            return Vec::new();
        }

        for (from, enemy) in self.enemies.iter().enumerate() {
            if enemy.health() == 0 {
                continue;
            }
            let target = self.decide_enemy_target(rng);
            let attack_damage =
                (enemy.info.attack as f64 * rng.random_range(0.9..1.1)).round() as u32;

            let health = &mut self.party[target].health;
            let limbs = health.targetable_limbs();
            let limb_index = limbs[rng.random_range(0..limbs.len())];
            let is_defending = self.party_defending[target];

            let prev_health = health[limb_index];
            if is_defending {
                let new_damage = attack_damage / 2;
                if health[limb_index] == 1 {
                    // coinflip
                    if rng.random_bool(0.5) {
                        health[limb_index] = 0;
                    }
                } else {
                    // leave with minimum 1 hp
                    health[limb_index] = health[limb_index].saturating_sub(new_damage).max(1);
                }
            } else {
                health[limb_index] = health[limb_index].saturating_sub(attack_damage);
            }
            let damage_dealt = prev_health.saturating_sub(health[limb_index]);
            if damage_dealt > 0 {
                self.party_attacked[target] += 1;
            }
            damage.push(DamageEvent {
                from,
                to: target,
                amount: damage_dealt,
            });
        }

        // reset defending state
        self.party_defending = [false; MAX_PARTY_SIZE];
        self.party_attacked = [0; MAX_PARTY_SIZE];

        damage
    }

    pub fn finish_enemy_turn(&mut self) {
        self.is_player_turn = true;
    }
}
