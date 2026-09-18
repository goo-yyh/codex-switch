---
title: 三步连接
description: 跟随界面截图添加 API Key、选择模型并开启 Codex Switch，了解如何在 Codex App 中使用模型和关闭服务。
---

准备好 Codex App、模型服务的 API Key 和可用额度。还没安装？查看[安装与下载](/docs/install/)。

**保存配置不会自动开启服务；开启后如需修改，先关闭服务。**

## 1. 添加服务和密钥

点击「新增配置」，选择厂商和对应套餐，填写配置名称与 API Key。普通服务保留预设地址；套餐或自定义地址的填写方式见[连接模型服务](/docs/providers/)。

[![新增配置：选择厂商、填写名称、地址与 API Key](/screenshots/connection.png)](/screenshots/connection.png)

_截图为应用浏览器预览，使用示例数据，连接状态为模拟。点击图片可放大。_

## 2. 选择模型并保存

勾选需要的模型，点击「保存配置」。列表没有的模型可输入准确 ID 添加；鼠标移到模型行后，可点击「设为默认」。一般保留预设能力即可。

[![勾选多个模型后保存配置](/screenshots/models.png)](/screenshots/models.png)

「测试配置」是可选操作：会请求所选模型，可能产生 API 费用，但不会保存配置。

## 3. 勾选配置并开启

首页勾选一个或多个配置，打开底部「Codex Switch」开关，再从系统中打开 Codex App。模型显示为 `配置名称-模型`。

[![首页勾选配置并开启底部服务开关](/screenshots/selected.png)](/screenshots/selected.png)

Codex 已在运行时，先完成当前任务，再正常退出并重新打开以加载模型。Codex Switch 不会自动重启 Codex。

开启后会显示锁定蒙层。**关闭蒙层中的服务开关，即可恢复原配置并继续编辑。** 详见[开启与关闭](/docs/switch/)。
