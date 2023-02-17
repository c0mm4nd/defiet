from pymongo import MongoClient
from os import listdir
from os.path import isfile, join
import csv
import logging
logging.basicConfig(
    format='%(asctime)s %(levelname)-8s %(message)s',
    level=logging.INFO,
    datefmt='%Y-%m-%d %H:%M:%S')

client = MongoClient('mongodb://%s:%s@172.24.1.2:2717' % ("lab0", "icaneatglass"))
db = client["DeFiET"]

folder = "csv_output"
for f in listdir("csv_output"):
    if not isfile(join(folder, f)):
        continue
    logging.warning("start for " + f)
    coll_name = f.replace(".csv", "")
    coll = db[coll_name]
    with open(join(folder, f)) as csvfile:
        reader = csv.DictReader(csvfile)
        for row in reader:
            coll.insert_one(row)
    logging.warning(f + "ends")