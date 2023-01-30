## AAVE v1

https://docs.aave.com/developers/v/1.0/integrating-aave/using-blockchain-data

https://docs.aave.com/developers/v/1.0/deployed-contracts/deployed-contract-instances

0x398eC7346DcD622eDc5ae82352F02bE94C62d119

事件：

```solidity
    /**
    * @dev emitted on deposit
    * @param _reserve the address of the reserve
    * @param _user the address of the user
    * @param _amount the amount to be deposited
    * @param _referral the referral number of the action
    * @param _timestamp the timestamp of the action
    **/
    event Deposit(
        address indexed _reserve,
        address indexed _user,
        uint256 _amount,
        uint16 indexed _referral,
        uint256 _timestamp
    );

    /**
    * @dev emitted during a redeem action.
    * @param _reserve the address of the reserve
    * @param _user the address of the user
    * @param _amount the amount to be deposited
    * @param _timestamp the timestamp of the action
    **/
    event RedeemUnderlying(
        address indexed _reserve,
        address indexed _user,
        uint256 _amount,
        uint256 _timestamp
    );

    /**
    * @dev emitted on borrow
    * @param _reserve the address of the reserve
    * @param _user the address of the user
    * @param _amount the amount to be deposited
    * @param _borrowRateMode the rate mode, can be either 1-stable or 2-variable
    * @param _borrowRate the rate at which the user has borrowed
    * @param _originationFee the origination fee to be paid by the user
    * @param _borrowBalanceIncrease the balance increase since the last borrow, 0 if it's the first time borrowing
    * @param _referral the referral number of the action
    * @param _timestamp the timestamp of the action
    **/
    event Borrow(
        address indexed _reserve,
        address indexed _user,
        uint256 _amount,
        uint256 _borrowRateMode,
        uint256 _borrowRate,
        uint256 _originationFee,
        uint256 _borrowBalanceIncrease,
        uint16 indexed _referral,
        uint256 _timestamp
    );

    /**
    * @dev emitted on repay
    * @param _reserve the address of the reserve
    * @param _user the address of the user for which the repay has been executed
    * @param _repayer the address of the user that has performed the repay action
    * @param _amountMinusFees the amount repaid minus fees
    * @param _fees the fees repaid
    * @param _borrowBalanceIncrease the balance increase since the last action
    * @param _timestamp the timestamp of the action
    **/
    event Repay(
        address indexed _reserve,
        address indexed _user,
        address indexed _repayer,
        uint256 _amountMinusFees,
        uint256 _fees,
        uint256 _borrowBalanceIncrease,
        uint256 _timestamp
    );

    /**
    * @dev emitted when a user performs a rate swap
    * @param _reserve the address of the reserve
    * @param _user the address of the user executing the swap
    * @param _newRateMode the new interest rate mode
    * @param _newRate the new borrow rate
    * @param _borrowBalanceIncrease the balance increase since the last action
    * @param _timestamp the timestamp of the action
    **/
    event Swap(
        address indexed _reserve,
        address indexed _user,
        uint256 _newRateMode,
        uint256 _newRate,
        uint256 _borrowBalanceIncrease,
        uint256 _timestamp
    );

    /**
    * @dev emitted when a user enables a reserve as collateral
    * @param _reserve the address of the reserve
    * @param _user the address of the user
    **/
    event ReserveUsedAsCollateralEnabled(address indexed _reserve, address indexed _user);

    /**
    * @dev emitted when a user disables a reserve as collateral
    * @param _reserve the address of the reserve
    * @param _user the address of the user
    **/
    event ReserveUsedAsCollateralDisabled(address indexed _reserve, address indexed _user);

    /**
    * @dev emitted when the stable rate of a user gets rebalanced
    * @param _reserve the address of the reserve
    * @param _user the address of the user for which the rebalance has been executed
    * @param _newStableRate the new stable borrow rate after the rebalance
    * @param _borrowBalanceIncrease the balance increase since the last action
    * @param _timestamp the timestamp of the action
    **/
    event RebalanceStableBorrowRate(
        address indexed _reserve,
        address indexed _user,
        uint256 _newStableRate,
        uint256 _borrowBalanceIncrease,
        uint256 _timestamp
    );

    /**
    * @dev emitted when a flashloan is executed
    * @param _target the address of the flashLoanReceiver
    * @param _reserve the address of the reserve
    * @param _amount the amount requested
    * @param _totalFee the total fee on the amount
    * @param _protocolFee the part of the fee for the protocol
    * @param _timestamp the timestamp of the action
    **/
    event FlashLoan(
        address indexed _target,
        address indexed _reserve,
        uint256 _amount,
        uint256 _totalFee,
        uint256 _protocolFee,
        uint256 _timestamp
    );

    /**
    * @dev these events are not emitted directly by the LendingPool
    * but they are declared here as the LendingPoolLiquidationManager
    * is executed using a delegateCall().
    * This allows to have the events in the generated ABI for LendingPool.
    **/

    /**
    * @dev emitted when a borrow fee is liquidated
    * @param _collateral the address of the collateral being liquidated
    * @param _reserve the address of the reserve
    * @param _user the address of the user being liquidated
    * @param _feeLiquidated the total fee liquidated
    * @param _liquidatedCollateralForFee the amount of collateral received by the protocol in exchange for the fee
    * @param _timestamp the timestamp of the action
    **/
    event OriginationFeeLiquidated(
        address indexed _collateral,
        address indexed _reserve,
        address indexed _user,
        uint256 _feeLiquidated,
        uint256 _liquidatedCollateralForFee,
        uint256 _timestamp
    );

    /**
    * @dev emitted when a borrower is liquidated
    * @param _collateral the address of the collateral being liquidated
    * @param _reserve the address of the reserve
    * @param _user the address of the user being liquidated
    * @param _purchaseAmount the total amount liquidated
    * @param _liquidatedCollateralAmount the amount of collateral being liquidated
    * @param _accruedBorrowInterest the amount of interest accrued by the borrower since the last action
    * @param _liquidator the address of the liquidator
    * @param _receiveAToken true if the liquidator wants to receive aTokens, false otherwise
    * @param _timestamp the timestamp of the action
    **/
    event LiquidationCall(
        address indexed _collateral,
        address indexed _reserve,
        address indexed _user,
        uint256 _purchaseAmount,
        uint256 _liquidatedCollateralAmount,
        uint256 _accruedBorrowInterest,
        address _liquidator,
        bool _receiveAToken,
        uint256 _timestamp
    );
```

https://docs.aave.com/developers/deployed-contracts/deployed-contracts


## AAVE v2

https://docs.aave.com/developers/v/2.0/deployed-contracts/deployed-contracts

https://docs.aave.com/developers/v/2.0/deployed-contracts/amm-market

## AAVE ARC

https://aave-arc.gitbook.io/docs/deployed-contracts/arc （？

## AAVE v3

跑在以太的L2，Optimism，建议暂时忽略
