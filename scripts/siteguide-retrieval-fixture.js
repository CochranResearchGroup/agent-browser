import assert from 'node:assert/strict';
import {writeFileSync} from 'node:fs';
import {join} from 'node:path';

const post=(id,title=id)=>`<article class="post" data-post-id="${id}"><h3>${title}</h3><a class="post-link" href="/r/fixture/comments/${id}">Post</a></article>`;
export function retrievalPage(url) {
 const parsed=new URL(url,'http://fixture.invalid');
 if(parsed.pathname!=='/reddit')return null;
 const mode=parsed.searchParams.get('case');
 if(mode==='empty')return '<main id="feed"></main>';
 if(mode==='none')return '<main id="feed"><p id="none">No results for this query</p></main>';
 if(mode==='login')return '<main id="login-wall">Sign in required</main>';
 if(mode==='blocked')return '<main id="blocked">Access blocked</main>';
 if(mode==='malformed')return '<main id="feed"><article class="post">Missing required post data</article></main>';
 if(mode==='bad-link')return '<main id="feed"><article class="post" data-post-id="broken"><h3>Missing link target</h3><a class="post-link">Post</a></article></main>';
 if(mode==='mixed')return '<main id="feed">'+post('good')+'<article class="post">Malformed</article></main>';
 if(mode==='scroll')return '<main id="feed">'+post('initial')+'</main><div style="height:3000px"></div><script>window.addEventListener("scroll",()=>{if(!document.querySelector("[data-post-id=late]"))document.querySelector("#feed").insertAdjacentHTML("beforeend",'+JSON.stringify(post('late'))+')});</script>';
 if(parsed.searchParams.get('page')==='2')return '<main id="feed">'+post('b')+post('c')+'</main>';
 return '<main id="feed">'+post('a')+post('b')+'</main><a rel="next" href="/reddit?page=2">Next</a>';
}

export async function runRetrievalFixture({call,software,taskId,directory,origin}) {
 const guideId='reddit-fixture';
 writeFileSync(join(directory,guideId+'.json'),JSON.stringify({schemaVersion:'agent-browser.siteguide.v1',guideId,name:'Owned Reddit feed',allowedOrigins:[origin],
  states:[{state:'login_required',selector:'#login-wall'},{state:'blocked',selector:'#blocked'}],
  retrieval:{containerSelector:'#feed',itemSelector:'.post',idAttribute:'data-post-id',titleSelector:'h3',urlSelector:'a.post-link',noResultsSelector:'#none',nextPageSelector:'a[rel=next]'}}));
 const navigate=path=>call('browser_navigate',{taskId,url:origin+path});
 const collect=options=>software.collectPosts({taskId,guideId,...options});
 await navigate('/reddit?page=1');const first=await collect({maxPages:2,maxItems:10});
 assert.equal(first.status,'completed');assert.deepEqual(first.items.map(item=>item.id),['a','b','c']);
 assert.equal(first.pagesUsed,2);assert.equal(first.taskId,taskId);
 await navigate('/reddit?page=1');const bounded=await collect({maxPages:1,maxItems:1});
 assert.equal(bounded.status,'partial');assert.equal(bounded.items.length,1);assert.equal(bounded.pagesUsed,1);
 await navigate('/reddit?case=none');const none=await collect();assert.equal(none.reasonCode,'no_results');assert.equal(none.noResultsVerified,true);assert.equal(none.items.length,0);
 for(const [mode,status,reason] of [['empty','adaptation_required','empty_extraction_unproven'],['malformed','adaptation_required','malformed_extraction'],['bad-link','adaptation_required','malformed_extraction'],['login','waiting','login_required'],['blocked','failed','site_blocked'],['mixed','partial','malformed_extraction']]) {
  await navigate('/reddit?case='+mode);const result=await collect();assert.equal(result.status,status);assert.equal(result.reasonCode,reason);assert.equal(result.noResultsVerified,false);
 }
 await navigate('/reddit?case=scroll');const scrolled=await collect({maxScrolls:1,maxItems:10});
 assert.equal(scrolled.scrollsUsed,1);assert(scrolled.items.some(item=>item.id==='late'));assert.equal(scrolled.status,'partial');
 console.log('Bounded normalized Reddit pages/scroll, verified no-results and malformed/login/blocked/partial states passed');
 await navigate('/');
}
