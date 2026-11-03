# NOT AN OFFICIAL MINECRAFT PRODUCT. NOT APPROVED BY OR ASSOCIATED WITH MOJANG OR MICROSOFT.
# (Это неофициальный продукт Minecraft, не одобрен Mojang или Microsoft и не связан с ними.

# ADR-0013 — Target game version 26.3

Status: ACCEPTED (2026-10-29)

GIBRID Native targets Minecraft Java Edition 26.3.

Protocol version 777 is a provisional value from project notes and is **not** verified against an installed game. Data Version must remain unset until extracted.

Code reads these values from `gibrid-version.toml`. Do not hard-code version literals in logic.
