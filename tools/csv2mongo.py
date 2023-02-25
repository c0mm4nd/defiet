from pymongo import MongoClient, InsertOne
from os import listdir
from os.path import isfile, join
import csv
from threading import Thread
import logging
logging.basicConfig(
    format='%(asctime)s %(levelname)-8s %(message)s',
    level=logging.INFO,
    datefmt='%Y-%m-%d %H:%M:%S')

client = MongoClient('mongodb://%s:%s@172.24.1.2:2717' % ("lab0", "icaneatglass"))
db = client["DeFiET"]

folder = "csv_output"
threads = []

def load(f):
    logging.warning("start for " + f)
    coll_name = f.replace(".csv", "")
    coll = db[coll_name]
    writes = []
    with open(join(folder, f)) as csvfile:
        reader = csv.DictReader(csvfile)
        for row in reader:
            writes.append(InsertOne(row))
    if len(writes) > 0:
        coll.bulk_write(writes)
    logging.warning(f + " ends")

for f in listdir("csv_output"):
    if not isfile(join(folder, f)):
        continue
    t = Thread(target=load, args=(f,))
    t.start()
    threads.append(t)

for t in threads:
    t.join()
