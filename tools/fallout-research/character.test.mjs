import assert from 'node:assert/strict';
import { characterSkill, characterStat, parseCharacter, SKILL, STAT, TRAIT } from './character.mjs';

function fixture() {
  const data = Buffer.alloc(428);
  let offset = 0;
  const write = (value) => { data.writeInt32BE(value, offset); offset += 4; };
  write(0x12345678);
  const base = Array.from({ length: 35 }, (_, i) => i < 7 ? 5 : (i === STAT.AGE ? 34 : 0));
  const bonuses = Array.from({ length: 35 }, () => 0);
  const skills = Array.from({ length: 18 }, () => 0);
  for (const value of base) write(value);
  for (const value of bonuses) write(value);
  for (const value of skills) write(value);
  write(2); // body type
  write(1234); // experience
  write(7); // kill type
  data.write('Vault Dweller', offset, 'ascii'); offset += 32;
  for (const tag of [SKILL.SMALL_GUNS, SKILL.SPEECH, -1, -1]) write(tag);
  for (const trait of [TRAIT.GIFTED, TRAIT.GOOD_NATURED]) write(trait);
  write(5); // character points
  assert.equal(offset, data.length);
  return data;
}

const bytes = fixture();
const character = parseCharacter(bytes);
assert.equal(character.flags, 0x12345678);
assert.equal(character.name, 'Vault Dweller');
assert.equal(character.nameBytes.length, 32);
assert.deepEqual(character.tags, [SKILL.SMALL_GUNS, SKILL.SPEECH, -1, -1]);
assert.deepEqual(character.traits, [TRAIT.GIFTED, TRAIT.GOOD_NATURED]);
assert.equal(character.bodyType, 2);
assert.equal(character.experience, 1234);
assert.equal(character.killType, 7);
assert.equal(character.characterPoints, 5);
assert.equal(character.raw.baseStats, character.baseStats);
assert.equal(character.metadata.bytes, 428);

assert.equal(characterStat(character, STAT.STRENGTH), 6); // Gifted
assert.equal(characterStat(character, STAT.MAXIMUM_ACTION_POINTS), 0);
assert.equal(characterStat(character, STAT.GENDER), 0);
assert.throws(() => characterStat(character, STAT.AGE), /Age requires options\.gameTime/);
assert.equal(characterStat(character, STAT.AGE, { gameTime: 365 * 24 * 60 * 60 * 10 }), 35);
const youngRawAge = { ...character, baseStats: [...character.baseStats] };
youngRawAge.baseStats[STAT.AGE] = 10;
assert.equal(characterStat(youngRawAge, STAT.AGE, { gameTime: 0 }), 16);
assert.throws(() => parseCharacter(Buffer.alloc(427)), /exactly 428 bytes/);
assert.throws(() => parseCharacter(new ArrayBuffer(428)), /Buffer or Uint8Array/);
assert.throws(() => characterStat(character, 35), /0 to 34/);

assert.equal(characterSkill(character, SKILL.SMALL_GUNS), 35 + 6 - 10 - 10 + 20); // base + AGI + Gifted + Good Natured + tag
assert.equal(characterSkill(character, SKILL.SPEECH, { gameDifficulty: 'normal' }), 25 + 12 - 10 + 15 + 20); // base + CHA + traits + tag
assert.throws(() => characterSkill(character, SKILL.FIRST_AID), /depends on game difficulty/);
assert.equal(characterSkill(character, SKILL.FIRST_AID, { gameDifficulty: 'easy' }), 30 + 6 - 10 + 15 + 20);

const nightPerson = { ...character, traits: [TRAIT.NIGHT_PERSON, -1] };
assert.throws(() => characterStat(nightPerson, STAT.PERCEPTION), /Night Person requires/);
assert.equal(characterStat(nightPerson, STAT.PERCEPTION, { gameTime: 264600 }), 4); // 07:21 applies daytime -1
assert.equal(characterStat(nightPerson, STAT.PERCEPTION, { gameTime: 0 }), 4); // literal source predicate also penalizes 00:00
assert.equal(characterStat(nightPerson, STAT.PERCEPTION, { gameTime: (5 * 60 + 59) * 600 }), 4); // and 05:59
assert.equal(characterStat(nightPerson, STAT.PERCEPTION, { gameTime: (17 * 60 + 59) * 600 }), 4); // 17:59 is still below threshold
assert.equal(characterStat(nightPerson, STAT.PERCEPTION, { gameTime: 18 * 60 * 600 }), 6); // 18:00 crosses exact HHMM threshold
assert.equal(characterSkill(nightPerson, SKILL.FIRST_AID, { gameDifficulty: 'normal', gameTime: 264600 }), 34);
assert.equal(characterSkill(nightPerson, SKILL.FIRST_AID, { gameDifficulty: 'normal', gameTime: 18 * 60 * 600 }), 36);
const finesse = { ...character, traits: [TRAIT.FINESSE, -1] };
assert.equal(characterStat(finesse, STAT.CRITICAL_CHANCE), 10);
const kamikaze = { ...character, traits: [TRAIT.KAMIKAZE, -1], baseStats: [...character.baseStats] };
kamikaze.baseStats[STAT.ARMOR_CLASS] = 12;
assert.equal(characterStat(kamikaze, STAT.ARMOR_CLASS), 0);
const resistance = { ...character, baseStats: [...character.baseStats] };
resistance.baseStats[STAT.DAMAGE_RESISTANCE_EMP] = 120;
resistance.baseStats[STAT.DAMAGE_RESISTANCE_EXPLOSION] = 120;
assert.equal(characterStat(resistance, STAT.DAMAGE_RESISTANCE_EMP), 100);
assert.equal(characterStat(resistance, STAT.DAMAGE_RESISTANCE_EXPLOSION), 90);
const fastMetabolism = { ...character, traits: [TRAIT.FAST_METABOLISM, -1], baseStats: [...character.baseStats] };
fastMetabolism.baseStats[STAT.RADIATION_RESISTANCE] = 30;
assert.equal(characterStat(fastMetabolism, STAT.RADIATION_RESISTANCE), 0);

console.log('Passed GCD parsing, initial stats, trait handling, supported skill calculations, and explicit missing-context errors.');
