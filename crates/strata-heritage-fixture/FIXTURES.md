# strata-heritage-fixture routes

A local stand-in for the 국가유산 인트라넷 (`intranet.gis-heritage.go.kr`), so no test and no
development run touches the real site. Element ids match the ones the ported scripts read
(`crates/strata-net/src/heritage_auto/scripts.rs`), taken from
`target/cpp-ref/src/core/HeritageIntranetFlow.cpp`.

`Fixture::start(account, city)` binds `127.0.0.1:0` and answers one request per connection.
Dropping the `Fixture` stops the listener.

| Method | Path | What it answers |
|---|---|---|
| GET | `/user/checkNet.do` | Before login: the login form (`#userId`, `#pwd`, `goLogin()`). After login: `<frameset>` whose `content` frame is `heritageDownload.do` and whose body carries `로그아웃`. |
| POST | `/j_spring_security_check` | Sets the session when `USER_ID`/`USER_PW` match the fixture's account, then answers the `checkNet.do` page. A wrong pair leaves the login form. |
| GET | `/main.do` | The frames page. |
| GET | `/user/data/heritageDownload.do` | The download panel: `#searchForm`, `#mode` = `S`, `#codedetaCd0`/`name=codedetaCd`, `#codeCdSg0`/`name=codeCdSg`, `#searchGisChaRirList`, `#downloadFilesAll`, `#tabContentDiv`, six `changeTab('S'|'P'|'B'|'U'|'R'|'E')` links, and a `pageSize` select with 「N개씩 보기」 options. The 지적 pair `#bjdcd1_newJbn` / `name=bjdCdNewAddr` comes **first in the DOM**, as on the site, so a region script that takes the first select pair fails here too. |
| GET/POST | `/user/data/heritageDownloadList.do` | Two rows and `검색결과 2 건`. |
| GET | `/user/data/heritageDownload/downloadFilesAll.do` | The ZIP given by `Fixture::give_zip`, or 404 when none was given. `Fixture::set_download_delay` makes it wait, standing in for the site's 40–120 s. |
| GET | `/favicon.ico` | 204. |

`Fixture::hits(path)` counts requests per path, so a test can prove a stage was reached.

## Where this fixture is freer than the site

- **Any city answers.** The fixture does not know which 시·군 the site would refuse; it serves
  the panel for whatever city it was started with, so a test can drive the region stage
  without a second fixture.
- **The site's RSA is absent.** `goLogin()` posts the plain form. The real page encrypts into
  hidden `USER_ID`/`USER_PW` fields; the fixture only needs the post to arrive.
- **The kind list does not change the rows.** Switching a tab sets `#mode` but the search
  answers the same two rows for every kind, so a run can be driven end to end without six
  different page shapes.
- **No tutorial popup.** The site shows one after login; `dismiss_tutorial_script` answers
  `none` here, which is the "nothing to dismiss" answer the machine already handles.
