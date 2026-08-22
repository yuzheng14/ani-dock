# Changelog

## [0.1.1](https://github.com/yuzheng14/ani-dock/compare/v0.1.0...v0.1.1) (2026-08-22)


### Features

* persist and serve cover images locally ([#44](https://github.com/yuzheng14/ani-dock/issues/44)) ([d8f1d95](https://github.com/yuzheng14/ani-dock/commit/d8f1d95663f937bae6ce994e65a955ecc5ff15dc)), closes [#7](https://github.com/yuzheng14/ani-dock/issues/7)


### Bug Fixes

* disable ANSI colors in Docker logs by default ([#55](https://github.com/yuzheng14/ani-dock/issues/55)) ([e055347](https://github.com/yuzheng14/ani-dock/commit/e0553476f112de48b7df9c41542aea09cef97308))
* distinguish unauthenticated and non-VIP accounts ([#54](https://github.com/yuzheng14/ani-dock/issues/54)) ([6cd6255](https://github.com/yuzheng14/ani-dock/commit/6cd6255ae5b986a5a17299da990a6938bab6a52b))
* handle structured API error responses ([#53](https://github.com/yuzheng14/ani-dock/issues/53)) ([8d107f6](https://github.com/yuzheng14/ani-dock/commit/8d107f6745ca2c2f2e2888429c4bb1d5502b4212))
* initialize data directory permissions ([#51](https://github.com/yuzheng14/ani-dock/issues/51)) ([f52df61](https://github.com/yuzheng14/ani-dock/commit/f52df6189f561745ffdac7c95215a3d497765604))
* use GHCR image by default in Compose ([#49](https://github.com/yuzheng14/ani-dock/issues/49)) ([9131ec7](https://github.com/yuzheng14/ani-dock/commit/9131ec7dbb42d75fae4ced21450089c5bb3d7303))

## 0.1.0 (2026-08-21)


### Features

* add ani-dock server crate ([80381af](https://github.com/yuzheng14/ani-dock/commit/80381afb82d9245350e04e4c27b5f6c6170abe81))
* add anime episode parsing ([de6a89e](https://github.com/yuzheng14/ani-dock/commit/de6a89e4a93059f0fe8c0514375554e6e39af20a))
* add anime import API ([e803baf](https://github.com/yuzheng14/ani-dock/commit/e803baff0bfe88cde57d57a133de3735bdbae5fa))
* add cookie file path ([6c27fd0](https://github.com/yuzheng14/ani-dock/commit/6c27fd026a1296e2d53fa660f6866bc1c79c5278))
* add cookie persistence ([3ac00e3](https://github.com/yuzheng14/ani-dock/commit/3ac00e3e089792e9907bf264c547c44fd599aee8))
* add database file path ([a0dd5ac](https://github.com/yuzheng14/ani-dock/commit/a0dd5ace96081ce72c5ea6691b2ce65373ab2e52))
* add episode bangumi directory paths ([20e9941](https://github.com/yuzheng14/ani-dock/commit/20e99413dcbeb1e26561b1b7c2c2bd65723bd26b))
* add episode download selection dialog ([5c7806e](https://github.com/yuzheng14/ani-dock/commit/5c7806ea18dd360a8adf12ca93c46861319901ac))
* add episode temporary directory paths ([57e0110](https://github.com/yuzheng14/ani-dock/commit/57e01103f0579be02fc62b4364cdcff4173bd16f))
* add frontend application shell ([1452c7e](https://github.com/yuzheng14/ani-dock/commit/1452c7ebbd70bc65fb353104124a38418b3ed77e))
* add frontend routing ([d20923f](https://github.com/yuzheng14/ani-dock/commit/d20923ff22ac7013e5b344fb80c2868ed21e9a6e))
* add persistent configuration support ([ce61605](https://github.com/yuzheng14/ani-dock/commit/ce61605a6b1f567686cdfd396791dd9aae2f7a07))
* add persistent download queue schema ([30b76c1](https://github.com/yuzheng14/ani-dock/commit/30b76c1175152e8f5ffec1a7620a19006d77e8ff))
* add settings API and frontend settings page ([e9ea79c](https://github.com/yuzheng14/ani-dock/commit/e9ea79ce099bf3439385c56c47b9b8fe3b50de4b))
* add SN list persistence ([1fd42c5](https://github.com/yuzheng14/ani-dock/commit/1fd42c53bf5d1571b6fff1268a34367840ec6450))
* complete anime parsing and download core ([73d41e6](https://github.com/yuzheng14/ani-dock/commit/73d41e6fecc4c111f4b64a6ab793996dcc1804be))
* define config type ([5746a00](https://github.com/yuzheng14/ani-dock/commit/5746a0021cd046293b20b7bc149eeb35e239cf3d))
* download and merge media segments ([3824dad](https://github.com/yuzheng14/ani-dock/commit/3824dadcddc229c59f2746f119a5d5cbe7b2df6d))
* expose episode download scheduling API ([bbfe8d1](https://github.com/yuzheng14/ani-dock/commit/bbfe8d1a53d03709934ca1ce5a8b722ca4924003))
* **frontend:** provide deployment-specific restart guidance ([#36](https://github.com/yuzheng14/ani-dock/issues/36)) ([a8794f9](https://github.com/yuzheng14/ani-dock/commit/a8794f97945be264b13cce6eb0abc0374dc7d571)), closes [#29](https://github.com/yuzheng14/ani-dock/issues/29)
* generate episode filenames ([5fbe497](https://github.com/yuzheng14/ani-dock/commit/5fbe497e600c5f6f141beefa62cba4db987fea66))
* implement anime database repository ([3a87da7](https://github.com/yuzheng14/ani-dock/commit/3a87da7c0111bcff9751281ffb2444a2b8ef522f))
* implement anime library import flow ([d0bb0c9](https://github.com/yuzheng14/ani-dock/commit/d0bb0c9d8bd77b1b25d19237011f3f5e93e4f4f7))
* make anime repository inserts idempotent ([5019512](https://github.com/yuzheng14/ani-dock/commit/50195129ec4876cfd44faba375d464e5b02643c6))
* make SN lists iterable ([59d90d8](https://github.com/yuzheng14/ani-dock/commit/59d90d863302207110dad6c23cf998ee74217250))
* normalize configuration limits ([fe89dad](https://github.com/yuzheng14/ani-dock/commit/fe89dad40a54ab48c6ad154168a9e80a2c773ea9))
* persist cookie updates through observable cookie jar ([2632f7b](https://github.com/yuzheng14/ani-dock/commit/2632f7bca135cca5ba6859d5ade4513cbd17e8ef))
* persist download queue and restore unfinished downloads ([2aebabb](https://github.com/yuzheng14/ani-dock/commit/2aebabba7aab98e18c965736806b221cbaaccbf1))
* report episode download progress ([6cc921b](https://github.com/yuzheng14/ani-dock/commit/6cc921b81dacec8f7b3a5f3223cf1db30c3a1ab0))
* sanitize path segments ([9b81812](https://github.com/yuzheng14/ani-dock/commit/9b818128dadecc65eaa95f2eae30df762cffda80))
* scaffold ani-dock db crate ([8eccc85](https://github.com/yuzheng14/ani-dock/commit/8eccc85e6ccfeb7b36942556c988b1336682ef9a))
* scaffold anime download pipeline ([45149b4](https://github.com/yuzheng14/ani-dock/commit/45149b4f65137ef720be2a80a749982d49667507))
* scaffold frontend workspace ([343d35a](https://github.com/yuzheng14/ani-dock/commit/343d35a4d7f141b249323b64a0e0adeccc86c86e))
* select media variant for download ([0fd07ba](https://github.com/yuzheng14/ani-dock/commit/0fd07ba0a2a6c45bed610af56e15a71577bc85e1))
* stream download progress to frontend via SSE ([64756b3](https://github.com/yuzheng14/ani-dock/commit/64756b329b60250ec430ffd1d7a72d14765edbe6))


### Bug Fixes

* add token response parse diagnostics ([961237e](https://github.com/yuzheng14/ani-dock/commit/961237e5bf21f76453ce3759c7d13a57f95aa380))
* **core:** migrate playlist resolution to the current API ([#34](https://github.com/yuzheng14/ani-dock/issues/34)) ([2c13dfd](https://github.com/yuzheng14/ani-dock/commit/2c13dfd8e2a5dbf0db9b535c4f2f4956235eb23e)), closes [#13](https://github.com/yuzheng14/ani-dock/issues/13)
* correct typos ([1ec0b89](https://github.com/yuzheng14/ani-dock/commit/1ec0b8929eed1cfc422c3ccd9663202735f352c5))
* default missing cookie file ([039736f](https://github.com/yuzheng14/ani-dock/commit/039736f9d5ff6193eeffc459dfb032c3c2951131))
* include license in Docker image ([#42](https://github.com/yuzheng14/ani-dock/issues/42)) ([e573758](https://github.com/yuzheng14/ani-dock/commit/e573758add12020aaf5154fb8e58df5754613a5f))
* initialize database and show anime names ([de25719](https://github.com/yuzheng14/ani-dock/commit/de25719f83e714938d50bf2d9055cf75eee7402d))
* prevent anime series name collisions ([5164116](https://github.com/yuzheng14/ani-dock/commit/5164116a9c0fe01060af461f6ed87ce893c14040))


### Build System

* add Docker and Docker Compose support ([#1](https://github.com/yuzheng14/ani-dock/issues/1)) ([3c35b3f](https://github.com/yuzheng14/ani-dock/commit/3c35b3f68a15c0a0fef9dba645fefb7c18ae880a))
