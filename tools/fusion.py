# %%
# pip install pyyaml
from os import listdir
from os.path import isfile, join
import argparse
from pymongo import MongoClient, InsertOne
import logging
import yaml

try:
    from yaml import CLoader as Loader, CDumper as Dumper
except ImportError:
    from yaml import Loader, Dumper


def load_config(path):
    with open(path) as f:
        return yaml.load(f, Loader)


# %%


class ProtocolFusionTask:
    pass


class FusionSet:
    fusion_coll_prefix = "fusion"

    def __init__(self, db):
        self.deopsit = db["deposit"]
        self.withdraw = db["withdraw"]
        self.borrow = db["borrow"]
        self.repay = db["repay"]
        self.liquidate = db["liquidate"]

    def load_assets(self, path):
        with open(path) as f:
            asset_list = yaml.load(f, Loader)
            assets = {}
            for main_asset_name, asset_info in asset_list.items():
                assets[asset_info["main"].lower()] = main_asset_name
                if len(asset_info["derivatives"]) == 0:
                    continue
                for derivative_name, derivative_addr in asset_info[
                    "derivatives"
                ].items():
                    assets[derivative_addr.lower()] = (
                        main_asset_name + "_" + derivative_name
                    )

        self.assets = assets
        return assets

    def parse_asset(self, doc, src_query):
        if src_query.startswith("0x"):
            return self.assets[src_query.lower()]
        else:
            val = self.query_value_from_mongo_doc(doc, src_query)
            return self.assets[val.lower()]

    def query_value_from_mongo_doc(self, doc, q: str):
        if q.startswith("eval("):
            this = doc
            command = q.removeprefix("eval(").removesuffix(")")
            rtn = eval(command)
        else:
            attr_chain = q.split(".")
            rtn = doc
            for attr in attr_chain:
                try:
                    attr = int(attr)
                except Exception as e:
                    pass
                rtn = rtn[attr]
        return rtn


    def handle_protocol_event_type(self, src_config):
        src_coll_name = f"{protocol_name}_{src_config['name']}"
        src_col_names = []

        aggregates = []
        for key in src_config.keys():
            if key.startswith("$"):
                # is a func
                func = key
                aggregates.append({key: src_config[key]})

            elif key in ["name"]:
                pass  # omit these
            else:
                src_col_names.append(key)

        # cols.extend()
        logging.warning(f"src: {src_coll_name}")
        src_coll = src_db[src_coll_name]

        # load $join config

        writes = []

        if len(aggregates) > 0:
            cursor = src_coll.aggregate(aggregates)
        else:
            cursor = src_coll.find()  # TODO: support more func

        for log in cursor:
            # logging.warning(log)
            row = {}

            for src_col_name in src_col_names:
                src_query = src_config[src_col_name]
                if src_query is None:
                    continue
                elif src_col_name == "asset":
                    row[src_col_name] = self.parse_asset(log, src_query)
                else:
                    row[src_col_name] = self.query_value_from_mongo_doc(log, src_query)

            # fixed items
            for src_col_name in [
                "block_number",
                "transaction_hash",
                "transaction_from",
                "transaction_to",
                "transaction_value",
                "contract",
                "gas_used",
                "priority_fee_per_gas",
                "effective_gas_price",
                "status",
            ]:
                row[src_col_name] = log[src_col_name]

            row["protocol"] = protocol_name

            writes.append(InsertOne(row))

        return writes

    def handle_protocol(self, protocol_name, protocol_config):
        logging.warning(f"start {protocol_name}")

        for fusion_type_name in protocol_config:
            # fusion_type_name: deposit withdraw ...
            logging.warning(f"start {fusion_type_name}")

            dst_coll = getattr(fusion, fusion_type_name)
            src_config = protocol_config[fusion_type_name]
            if src_config is None:
                logging.warning(
                    f"{protocol_name}.{fusion_type_name} is null, skipping..."
                )
                continue

            writes = self.handle_protocol_event_type(src_config)

            if len(writes) > 0:
                logging.warning(f"writing {len(writes)}")
                dst_coll.bulk_write(writes)


# %%

if __name__ == "__main__":
    logging.basicConfig(
        format="%(asctime)s %(levelname)-8s %(message)s",
        level=logging.INFO,
        datefmt="%Y-%m-%d %H:%M:%S",
    )

    parser = argparse.ArgumentParser(
        prog="fusion",
        description="What the program does",
        epilog="Text at the bottom of help",
    )
    parser.add_argument("-c", "--config", type=str, default="fusion.yml")
    parser.add_argument("--assets", type=str, default="assets.yml")
    parser.add_argument("-m", "--mongo", type=str, default="mongodb://127.0.0.1:27017")
    args = parser.parse_args()

    client = MongoClient(args.mongo)
    src_db = client["DeFiET"]
    dst_db = client["DeFiET-fusion"]

    fusion = FusionSet(dst_db)

    configs = []
    if isfile(args.config):
        configs.append(load_config(args.config))
    else:
        for f in listdir(args.config):
            path = join(args.config, f)
            if isfile(path):
                configs.append(load_config(path))

    fusion.load_assets(args.assets)

    for config in configs:
        for protocol_name in config.keys():
            fusion.handle_protocol(protocol_name, config[protocol_name])
