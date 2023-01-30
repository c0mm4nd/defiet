# 88mph

https://88mph.app

功能：
- 存款任意资产，（拿NFT作为凭证），得到MPH作为奖励
YT资产 -> 没用
- 收益令牌(例如$ cDAI-YT)使投机者可以从第三方协议的可变收益率上升或对冲其贷款成本的一部分中获利。例如. Compound的DAI借款人将用88mph购买cDAI-YT。.

产量令牌 Yield tokens：推测未来的可变利率收益率,并增强88mph协议的偿付能力

收益率令牌或YT是可替代的ERC-20 / ERC-1155令牌,使投机者可以从贷款协议(例如Compound或Aave)的可变收益率上升或对冲部分贷款借贷成本(例如. Compound的Dai借款人将以88英里/小时的速度购买cDAI YT。
当以88mph进行固定收益率存款时,用户可以购买YT,因为每个YT都与存款挂钩。期货交易赋予持有人权利,以赚取相应存款+期货交易的购买成本所产生的所有未来可变利率收益率。
原理。
期货交易不仅仅是推测收益的工具。.
88英里/小时本质上高度依赖于市场利率波动。. 假设一个存款人带来了1个具有固定条款的WBTC(3.75%/ 1年到期),然后基础贷款协议上的WBTC可变利率急剧下降(即从平均利率的10%降至1%)。. 然后,该协议需要找到2.75%的额外收益率,以确保到期时欠存款人的1.037 WBTC的可赎回性。.
知道以上内容后,该协议需要确保自己免受市场利率波动的影响。. 尽管浮动利率激增对协议有利,但如上例所示,下降幅度并不理想。.
因此,每小时88英里将这种波动性转移给其他参与者,这些参与者希望承受市场利率波动,而资本要求大大低于与收益率令牌相关的存款。. 因此,对于产量令牌持有人而言,可用的杠杆作用可能非常大(支付0.035即可在1.035上获得浮动利率)。.
总而言之,YT持有人是确保协议免受市场利率下降的代理人,并确保该协议始终有足够的准备金以及净利率以保持偿付能力。.

    另一个例子,我们可以。? 假设DAI-Compound固定APR资产的30天EMA为10%。. 因此,对于12个月的100 DAI存款,提供的固定APR为3.75%(30天EMA的37.5%)。. Cf固定利率模型部分。.
    相应的103.75 YT的购买成本为3.75 DAI(在下面了解有关YT令牌价格的更多信息)。. YT使持有人有权在存款期限内获得100 DAI本金+ 3.75 DAI赚取的可变利率收益率。. 如果平均复合变量APY在存款期间保持在10%,则YT向其持有人提供10.375 DAI。.
    假设您购买了所有103.75 YT。. 您的最终余额将为10.375 DAI。因此,您的投资为3.75 DAI,可赚取6.625 DAI利润,投资回报率为176.67%。.

产量支付。
每当提取部分或全部相应的存款时,都会自动触发向屈服令牌持有人的收益付款,并且还可以通过用户界面中的“索赔”按钮或调用合同函数DInterest.payInterestToFunders()来手动触发它。.
令牌持有人退款。
当在到期前提取一组收益率令牌的基础存款时,令牌持有人将获得退款,其金额是使用平均浮动收益率计算出的估计收益率的最低值,而固定收益率则提供了该收益率。提取的资金。. 如果完全提取了存款,则收益率令牌持有人将不再获得利息支付,并且当用户将来加满存款时,将创建新的收益率令牌合同。. 提早退出的可能性使收益率令牌的回报不确定性降低,从而使定价更加困难。.

v0 v1 v2 v3
https://docs.88mph.app/developer-docs/integration-guide

https://api.88mph.app/v2/pools

https://api.88mph.app/v3/pools

```

    // Events
    event EDeposit(
        address indexed sender,
        uint256 indexed depositID,
        uint256 depositAmount,
        uint256 interestAmount,
        uint256 feeAmount,
        uint64 maturationTimestamp
    );
    event ETopupDeposit(
        address indexed sender,
        uint64 indexed depositID,
        uint256 depositAmount,
        uint256 interestAmount,
        uint256 feeAmount
    );
    event ERolloverDeposit(
        address indexed sender,
        uint64 indexed depositID,
        uint64 indexed newDepositID
    );
    event EWithdraw(
        address indexed sender,
        uint256 indexed depositID,
        bool indexed early,
        uint256 virtualTokenAmount,
        uint256 feeAmount
    );
    event EFund(
        address indexed sender,
        uint64 indexed fundingID,
        uint256 fundAmount,
        uint256 tokenAmount
    );
    event EPayFundingInterest(
        uint256 indexed fundingID,
        uint256 interestAmount,
        uint256 refundAmount
    );
    event ESetParamAddress(
        address indexed sender,
        string indexed paramName,
        address newValue
    );
    event ESetParamUint(
        address indexed sender,
        string indexed paramName,
        uint256 newValue
    );
```

Factroy

```
    event CreateClone(
        string indexed contractName,
        address template,
        bytes32 salt,
        address clone
    );
```
