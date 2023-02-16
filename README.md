# DeFiET

**DeFi** **E**xtract & **T**ransform

This program will auto-fetch all DeFi logs and the log-related details into CSV files, you just need to edit one configuration file named `input.yml` to define the tasks.

## Build

1. Install rustlang & cargo toochain with rustup or system package manager
2. `git clone https://git.ngx.fi/c0mm4nd/defiet && cd defiet`
3. `cargo build --release`
4. The binary file is `./target/release/defiet`

## Usage

Write a `input.yml` config file as below:

```yml
parallel: false # Run all tasks in parallel or not
...             # and some other configs
sources:        # The contract source snippets of the task
  aave-v1:      # The task name, should be consistent with the name in `contracts:`
    events:     # All events of the contract. which are the most important parts for data extraction
      - Deposit(address indexed reverse, address indexed address , uint256 amount, uint16 indexed referral, uint256 timestamp)
      - RedeemUnderlying(address indexed reserve,address indexed user, uint256 amount, uint256 timestamp)
      - Borrow(address indexed _reserve,address indexed _user,    uint256 _amount,uint256 _borrowRateMode,uint256 _borrowRate,uint256 _originationFee,uint256 _borrowBalanceIncrease,        uint16 indexed _referral,uint256 _timestamp);
      - Repay(address indexed _reserve,  address indexed _user,address indexed _repayer,uint256 _amountMinusFees,uint256 _fees,uint256 _borrowBalanceIncrease,uint256 _timestamp);
  exact.ly:
    events:
      - Borrow(address indexed caller,address indexed receiver,address indexed borrower,uint256 assets,uint256 shares)
      - Repay(address indexed caller, address indexed borrower, uint256 assets, uint256 shares)
      - DepositAtMaturity(uint256 indexed maturity,address indexed caller,address indexed owner,uint256 assets,uint256 fee)
      - WithdrawAtMaturity(uint256 indexed maturity,address caller,address indexed receiver,address indexed owner,uint256 positionAssets,uint256 assets)
      - BorrowAtMaturity(uint256 indexed maturity,address caller,address indexed receiver,address indexed borrower,uint256 assets,uint256 fee)
      - RepayAtMaturity(uint256 indexed maturity,address indexed caller,address indexed borrower,uint256 assets,uint256 positionAssets)
      - Liquidate(address indexed receiver,address indexed borrower,uint256 assets,uint256 lendersAssets,address indexed seizeMarket,uint256 seizedAssets)
    structs:
contracts:                                       # The deployed contract addresses of the task
  aave-v1:                                       # The task name, should be consistent with the name in `sources:`
    - 0x398eC7346DcD622eDc5ae82352F02bE94C62d119 # The deployed contract addresses
  exact.ly:
    - 0xc4d4500326981eacD020e20A81b1c479c161c7EF
```

You can `cp input.example.yml input.yml` and `code input.yml` to start easy edit.

