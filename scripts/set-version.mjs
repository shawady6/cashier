// يضبط رقم الإصدار في tauri.conf.json وCargo.toml (يُستدعى من الـ workflow برقم البيلد).
import { readFileSync, writeFileSync } from "node:fs";

const version = process.argv[2];
if (!/^\d+\.\d+\.\d+$/.test(version ?? "")) {
  console.error("استخدم: node scripts/set-version.mjs MAJOR.MINOR.PATCH");
  process.exit(1);
}

const confPath = "src-tauri/tauri.conf.json";
const conf = JSON.parse(readFileSync(confPath, "utf8"));
conf.version = version;
writeFileSync(confPath, JSON.stringify(conf, null, 2) + "\n");

const cargoPath = "src-tauri/Cargo.toml";
const cargo = readFileSync(cargoPath, "utf8");
const next = cargo.replace(/^version = "[^"]*"/m, `version = "${version}"`);
if (next === cargo && !cargo.includes(`version = "${version}"`)) {
  console.error("لم يتم العثور على سطر الإصدار في Cargo.toml");
  process.exit(1);
}
writeFileSync(cargoPath, next);
console.log(`الإصدار: ${version}`);
