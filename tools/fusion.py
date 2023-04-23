# %%
# pip install pyyaml
import yaml
try:
    from yaml import CLoader as Loader, CDumper as Dumper
except ImportError:
    from yaml import Loader, Dumper
from os import listdir
from os.path import isfile, join
import argparse
from pymongo import MongoClient, InsertOne
import logging
logging.basicConfig(
    format='%(asctime)s %(levelname)-8s %(message)s',
    level=logging.INFO,
    datefmt='%Y-%m-%d %H:%M:%S')

parser = argparse.ArgumentParser(
                    prog = 'fusion',
                    description = 'What the program does',
                    epilog = 'Text at the bottom of help')
parser.add_argument('-c', '--config', type=str, default="fusion.yml")
parser.add_argument('--assets', type=str, default="assets.yml")
parser.add_argument('-m', '--mongo', type=str, default="mongodb://127.0.0.1:27017")
args = parser.parse_args()


client = MongoClient(args.mongo)
src_db = client["DeFiET"]
dst_db = client["DeFiET-fusion"]

def load_config(path):
    with open(path) as f:
        return yaml.load(f, Loader)

def load_assets(path):
    with open(path) as f:
        asset_list = yaml.load(f, Loader)
        assets = {}
        for main_asset_name, asset_info in asset_list.items():
            assets[asset_info["main"].lower()] = main_asset_name
            for derivative_name, derivative_addr in asset_info["derivatives"].items():
                assets[derivative_addr.lower()] = main_asset_name + "_" + derivative_name
        return assets


# %%

configs = []
if isfile(args.config):
    configs.append(load_config(args.config))
else:
    for f in listdir(args.config):
        path = join(args.config, f)
        if isfile(path):
            configs.append(load_config(path))

assets = load_assets(args.assets)
# %%

class FusionSet():
    fusion_coll_prefix = "fusion"
    def __init__(self, db):
        self.deopsit = db["deposit"]
        self.withdraw = db["withdraw"] 
        self.borrow = db["borrow"]
        self.repay = db["repay"]
        self.liquidate = db["liquidate"]

fusion = FusionSet(dst_db)

def handle_protocol(protocol_name, protocol_config):
    logging.warning(f"start {protocol_name}")
    for fusion_type_name in protocol_config:
        # fusion_type_name: deposit withdraw ...
        logging.warning(f"start {fusion_type_name}")

        dst_coll = getattr(fusion, fusion_type_name)
        src_config = protocol_config[fusion_type_name]
        if src_config is None:
            logging.warning(f"{protocol_name}.{fusion_type_name} is null, skipping...")
            continue
            return
        src_coll_name = f"{protocol_name}_{src_config['name']}"
        cols = list(src_config.keys())
        cols.remove("name")
        # cols.extend()
        logging.warning(f"src: {src_coll_name}")
        src_coll = src_db[src_coll_name]

        writes = []
        for log in src_coll.find():
            # logging.warning(log)
            row ={}
            for col in cols:
                src_col = src_config[col]
                if src_col is None:
                    continue
                if col == "asset": # not a fixed asset
                    if src_col.startswith("0x"):
                        row[col] = assets[src_col.lower()]
                    else:
                        row[col] = assets[log[src_col].lower()]
                else:
                    row[col] = log[src_col]

            for col in ["block_number", "transaction_hash", "transaction_from", "transaction_to", "contract"]:
                row[col] = log[col]

            row["protocol"] = protocol_name

            writes.append(InsertOne(row))

        if len(writes) > 0:
            logging.warning(f"writing {len(writes)}")
            dst_coll.bulk_write(writes)

for config in configs:
    for protocol_name in config.keys():
        handle_protocol(protocol_name, config[protocol_name])
