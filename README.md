# DeFiET

**DeFi** **E**xtract & **T**ransform

This program will auto-fetch all DeFi logs and the log-related details into CSV files, you just need to edit one configuration file named `input.yml` to define the tasks.

## Build

1. Install rustlang & cargo toochain with rustup or system package manager
2. `git clone https://git.ngx.fi/c0mm4nd/defiet && cd defiet`
3. `cargo build --release`
4. Run the binary file `./target/release/defiet --help`

## Usage

Write a `input.yml` config file as below:

```yml
aave-v1: # https://app-v1.aave.com
  contracts: # https://docs.aave.com/developers/v/1.0/deployed-contracts/deployed-contract-instances
    - 0x398eC7346DcD622eDc5ae82352F02bE94C62d119
  events:
    - event Deposit(address indexed _reserve,address indexed _user,uint256 _amount,uint16 indexed _referral,uint256 _timestamp)
    - event RedeemUnderlying(address indexed _reserve,address indexed _user,uint256 _amount,uint256 _timestamp)
    - event Borrow(address indexed _reserve,address indexed _user,uint256 _amount,uint256 _borrowRateMode,uint256 _borrowRate,uint256 _originationFee,uint256 _borrowBalanceIncrease,uint16 indexed _referral,uint256 _timestamp)
    - event Repay(address indexed _reserve,address indexed _user,address indexed _repayer,uint256 _amountMinusFees,uint256 _fees,uint256 _borrowBalanceIncrease,uint256 _timestamp)
    - event Swap(address indexed _reserve,address indexed _user,uint256 _newRateMode,uint256 _newRate,uint256 _borrowBalanceIncrease,uint256 _timestamp)
    - event ReserveUsedAsCollateralEnabled(address indexed _reserve, address indexed _user)
    - event ReserveUsedAsCollateralDisabled(address indexed _reserve, address indexed _user)
    - event RebalanceStableBorrowRate(address indexed _reserve,address indexed _user,uint256 _newStableRate,uint256 _borrowBalanceIncrease,uint256 _timestamp)
    - event FlashLoan(address indexed _target,address indexed _reserve,uint256 _amount,uint256 _totalFee,uint256 _protocolFee,uint256 _timestamp)
    - event LiquidationCall(address indexed _collateral,address indexed _reserve,address indexed _user,uint256 _purchaseAmount,uint256 _liquidatedCollateralAmount,uint256 _accruedBorrowInterest,address _liquidator,bool _receiveAToken,uint256 _timestamp)
```

You can `cp input.example.yml input.yml` and `code input.yml` to start easy edit.

`-p` flag will enable fetching **each protocol** data in **parallel**.
