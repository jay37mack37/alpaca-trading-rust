import re

with open("frontend/src/components/AgentsWorkspace.svelte", "r") as f:
    content = f.read()

# Add to getDescriptionForKind
content = content.replace(
    'case "vwap_reversion": return "VWAP Reversion: Standard deviation \'Snap Back\' strategy for over-extended price action.";',
    'case "vwap_reversion": return "VWAP Reversion: Standard deviation \'Snap Back\' strategy for over-extended price action.";\n      case "jarrod_vwap": return "Jarrod VWAP: Professional volume-confirmed VWAP reclaim strategy with integrated loss prevention.";'
)

# Add to labelForKind
content = content.replace(
    'case "vwap_reversion": return "VWAP Reversion";',
    'case "vwap_reversion": return "VWAP Reversion";\n      case "jarrod_vwap": return "Jarrod VWAP Reclaim";'
)

with open("frontend/src/components/AgentsWorkspace.svelte", "w") as f:
    f.write(content)
