# 智能ET设计

输入：
- 合约列表
- 合约类型
- 事件类型

输出
- csv？

首先从合约列表取出各个合约
合约根据类型确定需要的事件类型，如`"Deposit(address,address,uint256,uint16,uint256)"`
运行get_logs()，根据事件过滤，并根据