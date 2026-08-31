# Governed code

Every governed file must be an explicit regular file and must contain no more
than 250 physical lines. The checker receives explicit paths and does not
traverse a checkout or infer additional files.

The executable contract is maintained in
`plugins/codexy/skills/engineering/scripts/check_governed_code.py`.
