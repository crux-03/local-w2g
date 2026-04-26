import type { Snowflake } from "$lib/api/types";

export const compareSnowflake = (x: Snowflake, y: Snowflake): number => {
  const a = BigInt(x);
  const b = BigInt(y);
  return a < b ? -1 : a > b ? 1 : 0;
};
