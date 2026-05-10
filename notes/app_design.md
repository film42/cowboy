Cowboy... the online game!
==========================

Now that we have the rules defined (to the best of our ability), we want to make a collaborative online game so people can play cowboy together.

I'm thinking we can create a lobby where people can invite their friends with a link or a code (kind of like jacbox games). When they join, they can pick a name, and I want them to have video and audio chat available. Probably need to use WebRTC for this but they should be able to toggle over and see players kind of like zoom, even if they don't need to look there all the time.

Given it's so heavy in terms of multi-media, we want to make sure we're being as CPU efficient as possible. WE don't need the highest quality signals here or video all the time. The UX should be amazing. I know sometimes trying to screenshare on a jitsi meet can cRANK my cpu and I want to avoid that. I'm hoping to offload the A/V things that make background noise cancelation, echo cancelation, etc. just stuff that that makes the communication part great to the browsers or BEST in class open source software. I want the interface to feel magically light.

So... users create a lobby and can invite folks. Then the host (the one who creates the lobby, or who ends up with being the host if the current host passes it to them) can start a game. They can choose the number of lives, blocker tokens, and cowboy exemption tokens to be given to each person. If they choose one life, they do not get cowboy exemptions since everyone will just use that or die at the first cowboy round.

A game begins. The dealer chip is randomly assigned.

A round starts by dealing cards to the players. I don't think I want a traditional poker table view with all the players getting a card. That kind of sucks. Instead, maybe we have a view that cleanly shows the list of players (how many lives and tokens they have left), what you have in your hand, your lives, and blockers, and exemptions, and then what's currently happening and the view is somewhat dynamic to what's going on.

When in a normal round where we just trade, We see the current player, their lives and tokens, and then the next player so we can watch them goofing around and jestering about whether they should trade or not. (throughout the game, players can continue to chat by voice... we'll try to keep video focused on who is playing right now right now). So current player and next player is great for social funny actions. Then we animate the prev-next player to the current one and the new-next player slides into view.

Finally, we get to the dealer chip person. we see there is lable or icon showing them as the deal so it's the end, and next is just a deck of cards... they can choose to pass or take one off the top. If off the top, we all see the card.

If that's NOT a king, we go to the round judging view.

If it is a king, we start cowboy view.

Unwinding the stack here to the point where I said if it's not a cowboy round (no kings) then we do the trade stuff... whether it's cowboy from off the top or cowboy from the start, we should have an animation that goes COWBOY! And we see all the cowboy people on video. Then after like 2 seconds we go into the exemption voting where they have 5 seconds to vote if they want to exempt or not. Then people fold cards if they're exempt (list of players updates).. and then we launch in to a judgement of the round.

So judgement really looks to see who has the lowest card. Ace-high when in cowboy mode. Ace is lowest in normal mode.

We should probably keep track of round numbers, because that's fun... like "Round 5!" and eventually Round 5! Final four! ... Final 3, Final 2. Final Round! ... but of course, if we tie in surviving outcome then we trigger a resurrection! Where dead players come back to life and get one life. Tied players at the end keep what they had.

At the end of the game we show the winner and this becomes the "lobby" if you will... host can then start a new game or edit settings. The lobby can keep track of game results like Game 1, 10 players, 8 rounds, Jenna won.. Game 2, 8 players, 5 rounds, Gavin won. Etc. Lobby can have a results overview or whatever.

---

Stack is going to be a rust server with axum (probably?) and a react or preact frontend with vite. were going to end up deploying this on a VPS so we can use this a TURN server if we're using WebRTC. Most people will be playing on their phones.

People might disconnect accidentally, we should allow them to resume the session.

If an active player does not respond to their prompt within 15 seconds then we pick the default which is basically "yes, exempt", "pass", "don't block" . Maybe we'll allow the host to pause the game turns or boot a player if they're gone for good which would just cause them to lose their lives. Could be fun to eventually allow host to send a link to take over a player like if someone needs to stop but someone else can now play... maybe we punt that for now?

---

The design should focus on being radically simple. We'll have older people playing it. This is a game so button should be clear, choices to pass or trade should be simple. Animations should be smooth and crisp. It should feel fun.

I'm thinking generally bright colors. It shouldn't just feel like a stupid website, but a mobile game kind of. Jackbox still feels like I'm typing in a browser, and I want this to feel like a game.

We'll keep things simple for now. I'm prioritizing game play and collaboration with audio/ video. If that's dialed we can refine the UX but the game needs to match the rules.
