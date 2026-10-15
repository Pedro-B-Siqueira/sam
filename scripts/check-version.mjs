// Fails when a release tag does not match the version the app reports.
import { readFileSync } from 'node:fs';

const expected = process.argv[2];
if (!expected) {
  process.stderr.write('usage: node scripts/check-version.mjs <version>\n');
  process.exit(2);
}

const cargo = readFileSync('src-tauri/Cargo.toml', 'utf8').match(/^\[package\][^[]*?^version\s*=\s*"([^"]+)"/m)?.[1];
const versions = {
  'package.json': JSON.parse(readFileSync('package.json', 'utf8')).version,
  'src-tauri/tauri.conf.json': JSON.parse(readFileSync('src-tauri/tauri.conf.json', 'utf8')).version,
  'src-tauri/Cargo.toml': cargo,
};

const mismatched = Object.entries(versions).filter(([, version]) => version !== expected);
for (const [file, version] of mismatched) process.stderr.write(`${file} has version ${version ?? '(none)'}, the tag says ${expected}\n`);
process.exit(mismatched.length === 0 ? 0 : 1);
