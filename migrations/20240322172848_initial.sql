CREATE EXTENSION pg_trgm;

CREATE FUNCTION get_hue(username VARCHAR) RETURNS SMALLINT AS $$
    DECLARE
        bytes BYTEA := decode(left(md5(username), 4), 'hex');
    BEGIN
        RETURN (get_byte(bytes, 0) * 256 + get_byte(bytes, 1)) % 360;
    END;
$$ LANGUAGE plpgsql IMMUTABLE;

CREATE TABLE users(
    id SERIAL PRIMARY KEY,
    username VARCHAR NOT NULL UNIQUE,
    password_hash VARCHAR NOT NULL,
    is_admin BOOLEAN NOT NULL DEFAULT FALSE,
    has_avatar BOOLEAN NOT NULL DEFAULT FALSE,
    avatar_hue SMALLINT NOT NULL GENERATED ALWAYS AS (get_hue(username)) STORED
);


CREATE TABLE items(
    id SERIAL PRIMARY KEY,
    locator VARCHAR NOT NULL UNIQUE,
    title VARCHAR NOT NULL,
    description TEXT NOT NULL
);

CREATE TABLE reviews(
    id SERIAL PRIMARY KEY,
    item_id SERIAL NOT NULL REFERENCES items ON DELETE CASCADE,
    user_id SERIAL NOT NULL REFERENCES users ON DELETE CASCADE,
    rating SMALLINT NOT NULL,
    date TIMESTAMP NOT NULL DEFAULT now(),
    UNIQUE(item_id, user_id)
);

CREATE VIEW items_score AS SELECT i.*, COALESCE(AVG(r.rating)::REAL, 0) AS score, (SELECT COUNT(*) FROM reviews WHERE item_id=i.id) AS review_count, (DENSE_RANK() OVER (ORDER BY COALESCE(AVG(r.rating)::REAL, 0) DESC)) AS rank, (DENSE_RANK() OVER (ORDER BY (SELECT COUNT(*) FROM reviews WHERE item_id=i.id) DESC)) AS popularity FROM items i LEFT JOIN reviews r ON i.id=r.item_id GROUP BY i.id ORDER BY score DESC;

-- password = "password"
INSERT INTO users (username, password_hash, is_admin, has_avatar) VALUES ('admin','$argon2id$v=19$m=19456,t=2,p=1$yl6JrMcaYkmdt88DQceBvA$fP8L1jq0nhx+pX1170tkqZEEYEhQUVBdoasP5Gr/OVI',true,true);

INSERT INTO users (username, password_hash) VALUES ('test1','$argon2id$v=19$m=19456,t=2,p=1$yl6JrMcaYkmdt88DQceBvA$fP8L1jq0nhx+pX1170tkqZEEYEhQUVBdoasP5Gr/OVI'),
('test2','$argon2id$v=19$m=19456,t=2,p=1$yl6JrMcaYkmdt88DQceBvA$fP8L1jq0nhx+pX1170tkqZEEYEhQUVBdoasP5Gr/OVI'),
('test3','$argon2id$v=19$m=19456,t=2,p=1$yl6JrMcaYkmdt88DQceBvA$fP8L1jq0nhx+pX1170tkqZEEYEhQUVBdoasP5Gr/OVI'),
('test4','$argon2id$v=19$m=19456,t=2,p=1$yl6JrMcaYkmdt88DQceBvA$fP8L1jq0nhx+pX1170tkqZEEYEhQUVBdoasP5Gr/OVI'),
('test5','$argon2id$v=19$m=19456,t=2,p=1$yl6JrMcaYkmdt88DQceBvA$fP8L1jq0nhx+pX1170tkqZEEYEhQUVBdoasP5Gr/OVI'),
('test6','$argon2id$v=19$m=19456,t=2,p=1$yl6JrMcaYkmdt88DQceBvA$fP8L1jq0nhx+pX1170tkqZEEYEhQUVBdoasP5Gr/OVI');

INSERT INTO items (locator, title, description) VALUES ('ergo_proxy','Ergo Proxy',E'Within the domed city of Romdo lies one of the last human civilizations on Earth. Thousands of years ago, a global ecological catastrophe doomed the planet; now, life outside these domes is virtually impossible. To expedite mankind''s recovery, "AutoReivs," humanoid-like robots, have been created to assist people in their day-to-day lives. However, AutoReivs have begun contracting an enigmatic disease called the "Cogito Virus," which grants them self-awareness. Re-l Mayer, the granddaughter of Romdo''s ruler, is assigned to investigate this phenomenon alongside her AutoReiv partner, Iggy. But what begins as a routine investigation quickly spirals into a conspiracy as Re-l is confronted by humanity''s darkest sins.\n\nElsewhere in Romdo, an AutoReiv specialist by the name of Vincent Law must also face his demons when surreal events begin occurring around him. Re-l, Iggy, Vincent, and the child AutoReiv named Pino will form an unlikely faction as they struggle to uncover Romdo''s mysteries and discover the true purpose of the mythical beings called "Proxies."'),
('steins_gate','Steins;Gate',E'Eccentric scientist Rintarou Okabe has a never-ending thirst for scientific exploration. Together with his ditzy but well-meaning friend Mayuri Shiina and his roommate Itaru Hashida, Okabe founds the Future Gadget Laboratory in the hopes of creating technological innovations that baffle the human psyche. Despite claims of grandeur, the only notable "gadget" the trio have created is a microwave that has the mystifying power to turn bananas into green goo.\n\nHowever, when Okabe attends a conference on time travel, he experiences a series of strange events that lead him to believe that there is more to the "Phone Microwave" gadget than meets the eye. Apparently able to send text messages into the past using the microwave, Okabe dabbles further with the "time machine," attracting the ire and attention of the mysterious organization SERN.\n\nDue to the novel discovery, Okabe and his friends find themselves in an ever-present danger. As he works to mitigate the damage his invention has caused to the timeline, Okabe fights a battle to not only save his loved ones but also to preserve his degrading sanity.'),
('paranoia_agent','Paranoia Agent',E'The infamous Shounen Bat is terrorizing the residents of Musashino City. Flying around on his rollerblades and beating people down with a golden baseball bat, the assailant seems impossible to catch—much less understand. His first victim, the well-known yet timid character designer Tsukiko Sagi, is suspected of orchestrating the attacks. Believed only by her anthropomorphic pink stuffed animal, Maromi, Tsukiko is just one of Shounen Bat''s many victims.\n\nAs Shounen Bat continues his relentless assault on the town, detectives Keiichi Ikari and Mitsuhiro Maniwa begin to investigate the identity of the attacker. However, more and more people fall victim to the notorious golden bat, and news of the assailant begins circulating around the town. Paranoia starts to set in as chilling rumors spread amongst adults and children alike.\n\nWill the two detectives be able to unravel the truth behind Shounen Bat, or will the paranoia get to them first?'),
('chaos_head','ChäoS;HEAd',E'Throughout Shibuya, a series of murders dubbed the "New Generation Madness" gained widespread attention As these crimes gained infamy, they became a hot topic of discussion among the people of the area. Nonetheless, these "New Gen" murders do not capture the interest of Takumi Nishijou, a high school otaku who frequently experiences delusions and feels that he is constantly being watched.\n\nHaving no concern for the real world, Takumi spends his time playing online games and watching anime. However, his ordinary life is disrupted when he receives a horrifying image of a man staked to a wall from a user named Shogun. After calming himself at an internet cafe, Takumi sees the exact same murder scene as the image portrayed happen right before his eyes, along with a pink-haired girl covered in blood calling out his name.\n\nConflicted with the nature of reality, Takumi finds it difficult to judge where to place his trust as he gets caught up in the "New Gen" murders, believing that the murderer is out to get him.'),
('spirited_away','Spirited Away',E'Stubborn, spoiled, and naïve, 10-year-old Chihiro Ogino is less than pleased when she and her parents discover an abandoned amusement park on the way to their new house. Cautiously venturing inside, she realizes that there is more to this place than meets the eye, as strange things begin to happen once dusk falls. Ghostly apparitions and food that turns her parents into pigs are just the start—Chihiro has unwittingly crossed over into the spirit world. Now trapped, she must summon the courage to live and work amongst spirits, with the help of the enigmatic Haku and the cast of unique characters she meets along the way.\n\nVivid and intriguing, Sen to Chihiro no Kamikakushi tells the story of Chihiro''s journey through an unfamiliar world as she strives to save her parents and return home.'),
('psycho_pass','Psycho-Pass',E'Justice, and the enforcement of it, has changed. In the 22nd century, Japan enforces the Sibyl System, an objective means of determining the threat level of each citizen by examining their mental state for signs of criminal intent, known as their Psycho-Pass. Inspectors uphold the law by subjugating, often with lethal force, anyone harboring the slightest ill-will; alongside them are Enforcers, jaded Inspectors that have become latent criminals, granted relative freedom in exchange for carrying out the Inspectors'' dirty work.\n\nInto this world steps Akane Tsunemori, a young woman with an honest desire to uphold justice. However, as she works alongside veteran Enforcer Shinya Kougami, she soon learns that the Sibyl System''s judgments are not as perfect as her fellow Inspectors assume. With everything she has known turned on its head, Akane wrestles with the question of what justice truly is, and whether it can be upheld through the use of a system that may already be corrupt.'),
('bna','BNA',E'Throughout history, humans have been at odds with Beastmen—a species capable of changing shape due to their genetic "Beast Factor." Because of this conflict, Beastmen have been forced into hiding. Anima City serves as a safe haven for these oppressed individuals to live free from human interference.\n\nDuring a festival celebrating the town''s 10th anniversary, Michiru Kagemori, a human who suddenly turned into a tanuki, finds that Anima City is a far cry from paradise. After witnessing an explosion in the square, she is confronted by Shirou Ogami, a seemingly indestructible wolf and sworn protector of all Beastmen. As they pursue the criminals behind the bombing, the two discover that Michiru is anything but an ordinary Beastman, and look to investigate her mysterious past and uncanny abilities. Could she turn out to be the missing link between Humans and Beastmen?'),
('beastars','Beastars',E'In a civilized society of anthropomorphic animals, an uneasy tension exists between carnivores and herbivores. At Cherryton Academy, this mutual distrust peaks after a predation incident results in the death of Tem, an alpaca in the school''s drama club. Tem''s friend Legoshi, a grey wolf in the stage crew, has been an object of fear and suspicion for his whole life. In the immediate aftermath of the tragedy, he continues to lay low and hide his menacing traits, much to the disapproval of Louis, a red deer and the domineering star actor of the drama club.\n\nWhen Louis sneaks into the auditorium to train Tem''s replacement for an upcoming play, he assigns Legoshi to lookout duty. That very night, Legoshi has a fateful encounter with Haru, a white dwarf rabbit scorned by her peers. His growing feelings for Haru, complicated by his predatory instincts, force him to confront his own true nature, the circumstances surrounding the death of his friend, and the undercurrent of violence plaguing the world around him.'),
('bungou_stray_dogs','Bungou Stray Dogs',E'For weeks, Atsushi Nakajima''s orphanage has been plagued by a mystical tiger that only he seems to be unaware of. Suspected to be behind the strange incidents, the 18-year-old is abruptly kicked out of the orphanage and left hungry, homeless, and wandering through the city.\n\nWhile starving on a riverbank, Atsushi saves a rather eccentric man named Osamu Dazai from drowning. Whimsical suicide enthusiast and supernatural detective, Dazai has been investigating the same tiger that has been terrorizing the boy. Together with Dazai''s partner Doppo Kunikida, they solve the mystery, but its resolution leaves Atsushi in a tight spot. As various odd events take place, Atsushi is coerced into joining their firm of supernatural investigators, taking on unusual cases the police cannot handle, alongside his numerous enigmatic co-workers.'),
('flcl','FLCL',E'Naota Nandaba is an ordinary sixth grader living in a city where nothing amazing ever seems to happen. After his brother Tasuku leaves town to play baseball in America, Naota takes it upon himself to look after everything Tasuku left behind—from his top bunk bed to his ex-girlfriend Mamimi Samejima, who hasn''t stopped clinging to Naota since Tasuku left.\n\nLittle does Naota know, however, that his mundane existence is on the verge of being changed forever: enter Haruko Haruhara, a Vespa-riding, bass guitar-wielding, pink-haired psychopath whose first encounter with Naota leaves him with tire tracks on his back and a giant horn on his head. Though all he wants is some peace and quiet, when Haruko takes up residence at his parents'' home, Naota finds himself dragged into the heart of the greatest battle for supremacy that Earth—and quite possibly the entire universe—has ever seen.'),
('neon_genesis_evangelion','Neon Genesis Evangelion',E'Fifteen years after a cataclysmic event known as the Second Impact, the world faces a new threat: monstrous celestial beings called Angels invade Tokyo-3 one by one. Mankind is unable to defend themselves against the Angels despite utilizing their most advanced munitions and military tactics. The only hope for human salvation rests in the hands of NERV, a mysterious organization led by the cold Gendou Ikari. NERV operates giant humanoid robots dubbed "Evangelions" to combat the Angels with state-of-the-art advanced weaponry and protective barriers known as Absolute Terror Fields.\n\nYears after being abandoned by his father, Shinji Ikari, Gendou''s 14-year-old son, returns to Tokyo-3. Shinji undergoes a perpetual internal battle against the deeply buried trauma caused by the loss of his mother and the emotional neglect he suffered at the hands of his father. Terrified to open himself up to another, Shinji''s life is forever changed upon meeting 29-year-old Misato Katsuragi, a high-ranking NERV officer who shows him a free-spirited maternal kindness he has never experienced.\n\nA devastating Angel attack forces Shinji into action as Gendou reveals his true motive for inviting his son back to Tokyo-3: Shinji is the only child capable of efficiently piloting Evangelion Unit-01, a new robot that synchronizes with his biometrics. Despite the brutal psychological trauma brought about by piloting an Evangelion, Shinji defends Tokyo-3 against the angelic threat, oblivious to his father''s dark machinations.'),
('watamote','WataMote: No Matter How I Look At It, It''s You Guys'' Fault I''m Not Popular!',E'After living 50 simulated high school lives and dating over 100 virtual boys, Tomoko Kuroki believes that she is ready to conquer her first year of high school. Little does she know that she is much less prepared than she would like to think. In reality, Tomoko is an introverted and awkward young girl, and she herself is the only one who doesn''t realize it! With the help of her best friend, Yuu Naruse, and the support and love of her brother Tomoki, Tomoko attempts to brave the new world of high school life.\n\nWatashi ga Motenai no wa Dou Kangaetemo Omaera ga Warui! chronicles the life of a socially awkward and relatively friendless high school otaku as she attempts to overcome her personal barriers in order to live a fulfilling life.'),
('the_melancholy_of_haruhi_suzumiya','The Melancholy of Haruhi Suzumiya',E'If a survey were conducted to see if people believed in aliens, time travelers, or maybe espers, most would say they do not; average high school student Kyon considers himself among the non-believers. However, on his first day of school, he meets a girl who soon turns his world upside down.\n\nDuring class introductions, the beautiful Haruhi Suzumiya boldly announces her boredom with "normal" people and her intention of meeting supernatural beings. Dumbfounded, Kyon learns of her frustration with the lack of paranormal-focused clubs at their school and unwittingly inspires her to start her own club. She creates the Spreading Fun all Over the World with Haruhi Suzumiya Brigade, otherwise known as the SOS Brigade.\n\nFollowing the SOS Brigade''s founding, Haruhi manages to recruit Kyon and three other members: quiet bookworm Yuki Nagato, shy upperclassman Mikuru Asahina, and perpetually positive Itsuki Koizumi. Despite their normal appearance, the new members of the SOS Brigade each carry their own secrets related to Haruhi. Caught up in the mystery surrounding the eccentric club leader, Kyon is whisked away on a series of misadventures by Haruhi and the SOS Brigade, each one bringing him closer to the truth about who and what she is.');

INSERT INTO reviews(item_id, user_id, rating) VALUES (1, 1, 9),
(1,2,8),
(1,3,7),
(1,4,9),
(2,1,8),
(3,1,3),
(4,1,8);
