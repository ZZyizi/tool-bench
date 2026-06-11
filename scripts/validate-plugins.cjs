// Validate every plugin.json under plugins/ before codegen / dev / build.
//
// Checks:
//   1. plugin.json parses as JSON
//   2. Required fields present (id, name, version, category)
//   3. id is kebab-case and unique across plugins
//   4. category is one of the known buckets
//   5. Every commands[].argsRef / returnsRef references a type exported from src/types.ts
//      (regex match — good enough without a TS parser, catches typos)
//
// Exits non-zero on the first failure so npm run predev / prebuild abort early.

const { readFileSync, readdirSync, existsSync } = require('node:fs');
const { resolve, join } = require('node:path');

const ROOT = resolve(__dirname, '..');
const PLUGINS_DIR = join(ROOT, 'plugins');
const TYPES_PATH = join(ROOT, 'src', 'types.ts');

const REQUIRED_FIELDS = ['id', 'name', 'version', 'category'];
const KNOWN_CATEGORIES = ['Network', 'Encode', 'System', 'Other'];
const ID_PATTERN = /^[a-z][a-z0-9]*(-[a-z0-9]+)*$/;

function fail(msg) {
  console.error(`[validate-plugins] ${msg}`);
  process.exit(1);
}

function loadTypeNames() {
  if (!existsSync(TYPES_PATH)) {
    fail(`missing ${TYPES_PATH}`);
  }
  const src = readFileSync(TYPES_PATH, 'utf-8');
  const names = new Set();
  // Match `export interface Foo` / `export type Foo` — covers what argsRef/returnsRef can target.
  const re = /export\s+(?:interface|type)\s+([A-Za-z_][A-Za-z0-9_]*)/g;
  let m;
  while ((m = re.exec(src)) !== null) {
    names.add(m[1]);
  }
  return names;
}

function validatePlugin(dir, manifest, seenIds, knownTypes) {
  const where = `plugins/${dir}/plugin.json`;

  for (const f of REQUIRED_FIELDS) {
    if (manifest[f] === undefined || manifest[f] === null || manifest[f] === '') {
      fail(`${where}: missing required field "${f}"`);
    }
  }

  if (!ID_PATTERN.test(manifest.id)) {
    fail(`${where}: id "${manifest.id}" must be kebab-case (a-z, 0-9, '-')`);
  }
  if (seenIds.has(manifest.id)) {
    fail(`${where}: duplicate id "${manifest.id}" (already used by another plugin)`);
  }
  seenIds.add(manifest.id);

  if (!KNOWN_CATEGORIES.includes(manifest.category)) {
    fail(
      `${where}: category "${manifest.category}" must be one of ${KNOWN_CATEGORIES.join(', ')}`,
    );
  }

  const commands = manifest.commands ?? [];
  if (!Array.isArray(commands)) {
    fail(`${where}: commands must be an array`);
  }
  const seenCmdNames = new Set();
  for (const cmd of commands) {
    if (!cmd.name) fail(`${where}: command missing "name"`);
    if (seenCmdNames.has(cmd.name)) {
      fail(`${where}: duplicate command name "${cmd.name}"`);
    }
    seenCmdNames.add(cmd.name);
    for (const key of ['argsRef', 'returnsRef']) {
      const ref = cmd[key];
      if (ref === undefined || ref === 'void') continue;
      if (!knownTypes.has(ref)) {
        fail(
          `${where}: command "${cmd.name}" ${key} "${ref}" is not exported from src/types.ts`,
        );
      }
    }
  }
}

function main() {
  if (!existsSync(PLUGINS_DIR)) {
    console.log('[validate-plugins] no plugins/ directory; skipping');
    return;
  }
  const knownTypes = loadTypeNames();
  const seenIds = new Set();
  let count = 0;
  for (const dir of readdirSync(PLUGINS_DIR, { withFileTypes: true })) {
    if (!dir.isDirectory()) continue;
    const manifestPath = join(PLUGINS_DIR, dir.name, 'plugin.json');
    if (!existsSync(manifestPath)) continue;
    let manifest;
    try {
      manifest = JSON.parse(readFileSync(manifestPath, 'utf-8'));
    } catch (e) {
      fail(`plugins/${dir.name}/plugin.json: invalid JSON — ${e.message}`);
    }
    validatePlugin(dir.name, manifest, seenIds, knownTypes);
    count++;
  }
  console.log(`[validate-plugins] OK — ${count} plugin(s) validated`);
}

main();
