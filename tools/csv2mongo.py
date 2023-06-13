from pymongo import MongoClient, InsertOne
from os import listdir
from os.path import isfile, join
import csv
from threading import Thread
import logging

import argparse

parser = argparse.ArgumentParser(
    prog="fusion",
    description="What the program does",
    epilog="Text at the bottom of help",
)
parser.add_argument("-src", "--source", type=str, default="csv_output")
parser.add_argument("-m", "--mongo", type=str, default="mongodb://127.0.0.1:27017")


def load(db, f):
    logging.warning("start for " + f)
    coll_name = f.replace(".csv", "")
    coll = db[coll_name]
    writes = []
    with open(join(args.source, f)) as csvfile:
        reader = csv.DictReader(csvfile)
        for row in reader:
            writes.append(InsertOne(row))
    if len(writes) > 0:
        coll.bulk_write(writes)
    logging.warning(f + " ends")


if __name__ == "__main__":
    logging.basicConfig(
        format="%(asctime)s %(levelname)-8s %(message)s",
        level=logging.INFO,
        datefmt="%Y-%m-%d %H:%M:%S",
    )

    args = parser.parse_args()

    client = MongoClient(args.mongo)
    db = client["DeFiET"]

    threads = []

    for f in listdir(args.source):
        if not isfile(join(args.source, f)):
            continue
        t = Thread(target=load, args=(db, f))
        t.start()
        threads.append(t)

    for t in threads:
        t.join()
