import { helper } from "./helper.js";
import { missing } from "./missing.js";

export function run() {
  return helper() + missing();
}
