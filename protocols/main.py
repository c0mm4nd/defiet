# %% 
from web3 import Web3, WebsocketProvider

w3 = Web3(WebsocketProvider("ws://172.24.1.2:8545"))
w3.isConnected()
# %%
w3.eth.get_block('latest')

# %%
