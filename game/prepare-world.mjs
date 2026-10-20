// Prepare the first connected locations from the user's local installation.
import {execFileSync} from 'node:child_process';
import path from 'node:path';
if (!process.env.FALLOUT_GAME_PATH) throw Error('Set FALLOUT_GAME_PATH to your Fallout installation');
for (const map of ['v13ent', 'vault13', 'shadyw', 'shadye']) {
  execFileSync(process.execPath, [path.join(import.meta.dirname, 'prepare.mjs')], {
    env: {...process.env, FALLOUT_MAP: map}, stdio: 'inherit',
  });
}
