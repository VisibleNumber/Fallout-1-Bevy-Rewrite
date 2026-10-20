// Fallout 1 CE player-character GCD reader and initial-state calculations.
// Layout and numeric rules were checked against the pinned reference tree:
// reference/fallout1-ce/src/game/critter.cc (pc_load_data/critter_read_data),
// stat.cc (stat_level/stat_get_base/stat_recalc_derived), skill.cc
// (skill_data/skill_level), and trait.cc (trait_adjust_stat/trait_adjust_skill).

const GCD_LENGTH = 428;
const STAT_COUNT = 35;
const SKILL_COUNT = 18;
const TAG_COUNT = 4;
const TRAIT_COUNT = 2;
const NAME_LENGTH = 32;
const GAME_TIME_TICKS_PER_MINUTE = 600;
const GAME_TIME_TICKS_PER_YEAR = 365 * 24 * 60 * 60 * 10;

export const STAT = Object.freeze({
  STRENGTH: 0,
  PERCEPTION: 1,
  ENDURANCE: 2,
  CHARISMA: 3,
  INTELLIGENCE: 4,
  AGILITY: 5,
  LUCK: 6,
  MAXIMUM_HIT_POINTS: 7,
  MAXIMUM_ACTION_POINTS: 8,
  ARMOR_CLASS: 9,
  UNARMED_DAMAGE: 10,
  MELEE_DAMAGE: 11,
  CARRY_WEIGHT: 12,
  SEQUENCE: 13,
  HEALING_RATE: 14,
  CRITICAL_CHANCE: 15,
  BETTER_CRITICALS: 16,
  DAMAGE_THRESHOLD: 17,
  DAMAGE_THRESHOLD_LASER: 18,
  DAMAGE_THRESHOLD_FIRE: 19,
  DAMAGE_THRESHOLD_PLASMA: 20,
  DAMAGE_THRESHOLD_ELECTRICAL: 21,
  DAMAGE_THRESHOLD_EMP: 22,
  DAMAGE_THRESHOLD_EXPLOSION: 23,
  DAMAGE_RESISTANCE: 24,
  DAMAGE_RESISTANCE_LASER: 25,
  DAMAGE_RESISTANCE_FIRE: 26,
  DAMAGE_RESISTANCE_PLASMA: 27,
  DAMAGE_RESISTANCE_ELECTRICAL: 28,
  DAMAGE_RESISTANCE_EMP: 29,
  DAMAGE_RESISTANCE_EXPLOSION: 30,
  RADIATION_RESISTANCE: 31,
  POISON_RESISTANCE: 32,
  AGE: 33,
  GENDER: 34,
});

export const SKILL = Object.freeze({
  SMALL_GUNS: 0,
  BIG_GUNS: 1,
  ENERGY_WEAPONS: 2,
  UNARMED: 3,
  MELEE_WEAPONS: 4,
  THROWING: 5,
  FIRST_AID: 6,
  DOCTOR: 7,
  SNEAK: 8,
  LOCKPICK: 9,
  STEAL: 10,
  TRAPS: 11,
  SCIENCE: 12,
  REPAIR: 13,
  SPEECH: 14,
  BARTER: 15,
  GAMBLING: 16,
  OUTDOORSMAN: 17,
});

export const TRAIT = Object.freeze({
  FAST_METABOLISM: 0,
  BRUISER: 1,
  SMALL_FRAME: 2,
  ONE_HANDER: 3,
  FINESSE: 4,
  KAMIKAZE: 5,
  HEAVY_HANDED: 6,
  FAST_SHOT: 7,
  BLOODY_MESS: 8,
  JINXED: 9,
  GOOD_NATURED: 10,
  CHEM_RELIANT: 11,
  CHEM_RESISTANT: 12,
  NIGHT_PERSON: 13,
  SKILLED: 14,
  GIFTED: 15,
});

const SKILL_FORMULAS = [
  [35, 1, 5, -1], [10, 1, 5, -1], [10, 1, 5, -1],
  [65, 1, 5, 0], [55, 1, 5, 0], [40, 1, 5, -1],
  [30, 1, 1, 4], [15, 1, 1, 4], [25, 1, 5, -1],
  [20, 1, 1, 5], [20, 1, 5, -1], [20, 1, 1, 5],
  [25, 2, 4, -1], [20, 1, 4, -1], [25, 2, 3, -1],
  [20, 2, 3, -1], [20, 3, 6, -1], [5, 1, 2, 4],
];

const DIFFICULTY_SENSITIVE_SKILLS = new Set([
  SKILL.FIRST_AID, SKILL.DOCTOR, SKILL.SNEAK, SKILL.LOCKPICK,
  SKILL.STEAL, SKILL.TRAPS, SKILL.SCIENCE, SKILL.REPAIR,
  SKILL.SPEECH, SKILL.BARTER, SKILL.GAMBLING, SKILL.OUTDOORSMAN,
]);

const STAT_LIMITS = [
  ...Array.from({ length: 7 }, () => [1, 10]),
  [0, 0x7fffffff], [0, 0x7fffffff], [0, 0x7fffffff],
  [0, 0x7fffffff], [0, 0x7fffffff], [0, 0x7fffffff], [0, 0x7fffffff],
  [0, 0x7fffffff], [0, 100], [-60, 100],
  ...Array.from({ length: 7 }, () => [0, 100]),
  [0, 90], [0, 90], [0, 90], [0, 90], [0, 90], [0, 100], [0, 90],
  [0, 100], [0, 100], [16, 35], [0, 1],
];

function asBytes(input) {
  if (Buffer.isBuffer(input)) return input;
  if (input instanceof Uint8Array) return Buffer.from(input.buffer, input.byteOffset, input.byteLength);
  throw new TypeError('Character data must be a Buffer or Uint8Array');
}

function traitHas(character, trait) {
  return character.traits.includes(trait);
}

function checkedIndex(index, length, label) {
  if (!Number.isInteger(index) || index < 0 || index >= length) {
    throw new RangeError(`${label} index must be an integer from 0 to ${length - 1}`);
  }
}

/** Parse the fixed 428-byte player-character GCD record; integers are BE as in db_freadInt. */
export function parseCharacter(input) {
  const bytes = asBytes(input);
  if (bytes.length !== GCD_LENGTH) {
    throw new RangeError(`A Fallout 1 player GCD must be exactly ${GCD_LENGTH} bytes; got ${bytes.length}`);
  }
  let offset = 0;
  const readInt = () => {
    const value = bytes.readInt32BE(offset);
    offset += 4;
    return value;
  };
  const flags = readInt();
  const baseStats = Array.from({ length: STAT_COUNT }, readInt);
  const bonusStats = Array.from({ length: STAT_COUNT }, readInt);
  const skills = Array.from({ length: SKILL_COUNT }, readInt);
  const bodyType = readInt();
  const experience = readInt();
  const killType = readInt();
  const nameBytes = Buffer.from(bytes.subarray(offset, offset + NAME_LENGTH));
  offset += NAME_LENGTH;
  const nameEnd = nameBytes.indexOf(0);
  const name = new TextDecoder('windows-1252').decode(nameBytes.subarray(0, nameEnd < 0 ? nameBytes.length : nameEnd));
  const tags = Array.from({ length: TAG_COUNT }, readInt);
  const traits = Array.from({ length: TRAIT_COUNT }, readInt);
  const characterPoints = readInt();
  if (offset !== GCD_LENGTH) throw new Error('Internal GCD layout error');

  return {
    flags,
    baseStats,
    bonusStats,
    skills,
    bodyType,
    experience,
    killType,
    name,
    nameBytes,
    tags,
    traits,
    characterPoints,
    raw: { flags, baseStats, bonusStats, skills, bodyType, experience, killType, nameBytes, tags, traits, characterPoints },
    metadata: {
      format: 'Fallout 1 CE player GCD',
      bytes: GCD_LENGTH,
      calculatedValues: 'Initial static values only; no inventory, active combat, temporary condition, perk, elapsed game time, or user difficulty configuration is represented in this file.',
      statLimits: 'Age is computed when gameTime is supplied; current HP/poison/radiation remain runtime state.',
      skillLimits: 'Perk and difficulty adjustments are not represented; difficulty-sensitive skill queries require an explicit difficulty option.',
    },
  };
}

function gameTimeHour(gameTime) {
  if (!Number.isSafeInteger(gameTime) || gameTime < 0 || gameTime > 0x7fffffff) {
    throw new TypeError('Night Person requires options.gameTime as non-negative signed 32-bit game ticks');
  }
  const minutes = Math.trunc(gameTime / GAME_TIME_TICKS_PER_MINUTE);
  return 100 * (Math.trunc(minutes / 60) % 24) + (minutes % 60);
}

function statTraitAdjustment(character, statIndex, options) {
  let adjustment = 0;
  if (traitHas(character, TRAIT.GIFTED) && statIndex >= STAT.STRENGTH && statIndex <= STAT.LUCK) adjustment++;
  switch (statIndex) {
    case STAT.STRENGTH:
      if (traitHas(character, TRAIT.BRUISER)) adjustment += 2;
      break;
    case STAT.PERCEPTION:
    case STAT.INTELLIGENCE:
      if (traitHas(character, TRAIT.NIGHT_PERSON)) {
        const hour = gameTimeHour(options.gameTime);
        // Preserve trait_adjust_stat's literal HHMM test, including its
        // pre-18:00 penalty at midnight and throughout the early morning.
        adjustment += hour - 600 < 1200 ? -1 : 1;
      }
      break;
    case STAT.AGILITY:
      if (traitHas(character, TRAIT.SMALL_FRAME)) adjustment++;
      break;
    case STAT.MAXIMUM_ACTION_POINTS:
      if (traitHas(character, TRAIT.BRUISER)) adjustment -= 2;
      break;
    case STAT.ARMOR_CLASS:
      if (traitHas(character, TRAIT.KAMIKAZE)) adjustment -= character.baseStats[STAT.ARMOR_CLASS];
      break;
    case STAT.MELEE_DAMAGE:
      if (traitHas(character, TRAIT.HEAVY_HANDED)) adjustment += 4;
      break;
    case STAT.CARRY_WEIGHT:
      if (traitHas(character, TRAIT.SMALL_FRAME)) adjustment -= 10 * character.baseStats[STAT.STRENGTH];
      break;
    case STAT.SEQUENCE:
      if (traitHas(character, TRAIT.KAMIKAZE)) adjustment += 5;
      break;
    case STAT.HEALING_RATE:
      if (traitHas(character, TRAIT.FAST_METABOLISM)) adjustment += 2;
      break;
    case STAT.CRITICAL_CHANCE:
      if (traitHas(character, TRAIT.FINESSE)) adjustment += 10;
      break;
    case STAT.BETTER_CRITICALS:
      if (traitHas(character, TRAIT.HEAVY_HANDED)) adjustment -= 30;
      break;
    case STAT.RADIATION_RESISTANCE:
    case STAT.POISON_RESISTANCE:
      if (traitHas(character, TRAIT.FAST_METABOLISM)) adjustment -= character.baseStats[statIndex];
      break;
  }
  return adjustment;
}

/** Return stat_level for a file-backed stat; gameTime supplies dynamic time-dependent effects. */
export function characterStat(character, statIndex, options = {}) {
  checkedIndex(statIndex, STAT_COUNT, 'Stat');
  if (!character || !Array.isArray(character.baseStats) || character.baseStats.length !== STAT_COUNT ||
      !Array.isArray(character.bonusStats) || character.bonusStats.length !== STAT_COUNT ||
      !Array.isArray(character.traits) || character.traits.length !== TRAIT_COUNT) {
    throw new TypeError('Expected a parsed Fallout 1 character');
  }
  const [minimum, maximum] = STAT_LIMITS[statIndex];
  let value = character.baseStats[statIndex] + statTraitAdjustment(character, statIndex, options) + character.bonusStats[statIndex];
  if (statIndex === STAT.AGE) {
    if (!Number.isSafeInteger(options.gameTime) || options.gameTime < 0 || options.gameTime > 0x7fffffff) {
      throw new TypeError('Age requires options.gameTime as non-negative signed 32-bit game ticks');
    }
    value += Math.trunc(options.gameTime / GAME_TIME_TICKS_PER_YEAR);
  }
  return Math.max(minimum, Math.min(maximum, value));
}

/** Return skill_level for an initial character. options.gameDifficulty may be normal/easy/hard. */
export function characterSkill(character, skillIndex, options = {}) {
  checkedIndex(skillIndex, SKILL_COUNT, 'Skill');
  if (!character || !Array.isArray(character.skills) || character.skills.length !== SKILL_COUNT ||
      !Array.isArray(character.tags) || character.tags.length !== TAG_COUNT ||
      !Array.isArray(character.traits) || character.traits.length !== TRAIT_COUNT) {
    throw new TypeError('Expected a parsed Fallout 1 character');
  }
  const [base, statModifier, stat1, stat2] = SKILL_FORMULAS[skillIndex];
  const statBonus = stat2 < 0
    ? characterStat(character, stat1, options) * statModifier
    : Math.trunc((characterStat(character, stat1, options) + characterStat(character, stat2, options)) * statModifier / 2);
  const points = character.skills[skillIndex];
  let value = base + statBonus + points;
  if (character.tags.includes(skillIndex)) value += 20 + points;
  if (traitHas(character, TRAIT.GIFTED)) value -= 10;
  if (traitHas(character, TRAIT.SKILLED)) value += 10;
  if (traitHas(character, TRAIT.GOOD_NATURED)) {
    if (skillIndex <= SKILL.THROWING) value -= 10;
    else if ([SKILL.FIRST_AID, SKILL.DOCTOR, SKILL.SPEECH, SKILL.BARTER].includes(skillIndex)) value += 15;
  }
  if (DIFFICULTY_SENSITIVE_SKILLS.has(skillIndex)) {
    const difficulty = options.gameDifficulty;
    if (!['normal', 'easy', 'hard'].includes(difficulty)) {
      throw new Error('This skill depends on game difficulty; pass options.gameDifficulty as normal, easy, or hard');
    }
    if (difficulty === 'easy') value += 20;
    if (difficulty === 'hard') value -= 10;
  }
  return Math.min(value, 200);
}
