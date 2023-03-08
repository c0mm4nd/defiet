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

parser = argparse.ArgumentParser(
                    prog = 'fusion',
                    description = 'What the program does',
                    epilog = 'Text at the bottom of help')
parser.add_argument('-c', '--config', type=str, default="fusion.config")
args = parser.parse_args()

def load_config(path):
    with open(path) as f:
        return yaml.load(f, Loader)

configs = []
if isfile(parser.config):
    configs.append(load_config(args.config))
else:
    for f in listdir(args.parser):
        path = join(args.config, f)
        if isfile(path):
            configs.append(load_config(path))
# %%
fusion_coll_name = "fusion"

for config in configs:
    for protocol_name in config.keys():
        protocol_config = config[protocol_name]
        for event_name in protocol_config:
            coll_name = f"{protocol_name}_{event_name}"
            