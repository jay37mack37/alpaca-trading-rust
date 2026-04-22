with open("frontend/src/lib/types.ts", "r") as f:
    content = f.read()

content = content.replace(
    '"vwap_reversion";',
    '"vwap_reversion" | "jarrod_vwap";'
)

with open("frontend/src/lib/types.ts", "w") as f:
    f.write(content)
