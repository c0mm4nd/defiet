#%%
import requests
import json

resp = requests.get("https://api.llama.fi/protocols")
protocols = json.loads(resp.text)
protocols
# %%
# categories = set()
# for protocol in protocols:
#     cat = protocol["category"]
#     categories.add(cat)
# categories

# %%
def is_eth_lending(p):
    return "Ethereum" in p["chains"] and p["category"] == "Lending" # in ["Lending", ]
all_targets = list(filter(is_eth_lending, protocols))
all_targets

#%%
def is_inno_eth_lending(p):
    return (p.get("forkedFrom") is None or len(p.get("forkedFrom")) == 0) and "Ethereum" in p["chains"] and p["category"] == "Lending" and p["chainTvls"]["Ethereum"] > 1_000_000 # in ["Lending", ]
inno_targets = list(filter(is_inno_eth_lending, protocols))
inno_targets
sorted(inno_targets, key=lambda p: p["chainTvls"]["Ethereum"])

# %%
f = open("targets.json", "w")
json.dump(all_targets, f, indent=4)
f.close()

# %%
f = open("inno_targets.json", "w")
json.dump(inno_targets, f, indent=4)
f.close()
# %%
names = list(map(lambda p: p["name"], inno_targets))
names.sort()
names
# %%
