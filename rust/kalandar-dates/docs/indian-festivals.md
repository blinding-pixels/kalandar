# Indian lunisolar festivals

date-holidays 3.37.0 does not compute Hindu festivals. Its India data has only the fixed national days, Good Friday and Christmas. So the build-time generator in `tools/holidays/` computes the major Hindu festivals itself, for every year from 1970 to 2100, and adds them to the India region of the embedded data. Nothing is computed at runtime: the crate only reads the dates the generator wrote.

## Astronomy

My own JavaScript routines in `tools/holidays/astro.mjs` do the calculation. No Swiss Ephemeris or AGPL code is used.

- **Sun:** the apparent longitude from Meeus, *Astronomical Algorithms*, chapter 25 (low-accuracy series, plus nutation and aberration).
- **Moon:** the longitude from Meeus chapter 47, using the 59 main periodic terms of table 47.A and the three additive terms.
- **Accuracy:** about 0.01° for the Sun and better than 0.01° for the Moon. That puts tithi boundaries within a few minutes, and sankranti times within about 20 minutes of published panchangs.
- **ΔT:** the Espenak–Meeus polynomials.
- **Sidereal longitudes:** use the Lahiri (Chitrapaksha) ayanamsa, taken as 23.85303° at J2000 plus 50.2788″ a year.
- **Sunrise and sunset:** the Sun's upper limb at −0.8333°, with no correction for local terrain.
- **Reference places:** New Delhi (28.6139 N, 77.2090 E) for most festivals, Chennai for Pongal, and Thiruvananthapuram for Onam. All times are IST.

## Calendar conventions

- **Tithi:** the Moon–Sun elongation divided into 30 parts of 12°. Tithis 1–15 are the waxing half (shukla), 16–30 the waning half (krishna). Tithi 30 is amavasya and tithi 15 is purnima.
- **Lunar months:** amanta, running from new moon to new moon. A month is named from the sidereal sign the Sun is in at the new moon that starts it. Sun in Meena gives Chaitra, Mesha gives Vaishakha, and so on to Kumbha giving Phalguna.
  - A month with no sankranti, where both new moons fall in the same sign, is adhika. No festival is placed in an adhika month.
  - A month with two sankrantis is kshaya, and stands for both of the months it covers. This happened in 1983.
- **Purnimanta regions:** much of North India names the dark half after the next month. That changes the month's name, not the day. Each festival below is defined by an amanta month and tithi, which gives the same calendar day in either system.
- **The day of a tithi:** the civil day picked by the rule given for each festival. The default is the udaya rule: the day whose sunrise falls inside the tithi. If no sunrise falls inside it (a kshaya tithi), the day it begins is used.

## Festivals and their rules

| Festival | Amanta month and tithi | Day rule |
|---|---|---|
| Diwali (Lakshmi Puja) | Ashvin amavasya | the day whose pradosh (sunset to 2 h 24 min after) has the most amavasya |
| Holi (Dhulandi) | the day after Holika Dahan | Holika Dahan is Phalguna purnima at pradosh, free of bhadra (the first half of purnima). If day 1's pradosh is all bhadra, it is on day 2 when purnima lasts at least 3/4 of day 2's daylight, and otherwise on day 1. |
| Dussehra (Vijayadashami) | Ashvin shukla 10 | the day with dashami in aparahna (the 4th fifth of daylight). If both days have it, the one with Shravana nakshatra in aparahna, and otherwise the first. If only one day has it, and only partly (under 85%), the next day is used when dashami is at its sunrise and Shravana is in its aparahna. |
| Janmashtami | Shravana krishna 8 (Bhadrapada in purnimanta) | udaya, the Vaishnava convention the central list follows |
| Ganesh Chaturthi | Bhadrapada shukla 4 | the day whose madhyahna (the 3rd fifth of daylight) has the most chaturthi |
| Maha Shivaratri | Magha krishna 14 (Phalguna in purnimanta) | nishita (48 minutes around midnight). If both nights have it, the later one. |
| Raksha Bandhan | Shravana purnima | udaya, provided purnima lasts at least 6 ghatis (2 h 24 min) after that sunrise; otherwise the day before |
| Ram Navami | Chaitra shukla 9 | madhyahna. If both days have it, the later one. |
| Navratri (Sharad), first day | Ashvin shukla 1 | udaya |
| Chaitra Navratri, first day | Chaitra shukla 1 | udaya |
| Karva Chauth | Ashvin krishna 4 (Kartika in purnimanta) | the day whose evening (1 to 4 h after sunset, around moonrise) has the most chaturthi |
| Makar Sankranti | the Sun enters sidereal Makara (270°) | the sunrise-to-sunrise day that contains the moment |
| Pongal (Thai Pongal) | the same moment, Chennai | that civil day if the moment comes before the last fifth of daylight, otherwise the next day |
| Onam (Thiruvonam) | Shravana nakshatra in the solar month Chingam (the Sun in sidereal Simha) | Chingam 1 is the day of the Simha sankranti, or the next day if it comes after 3/5 of daylight. Onam is the first day from Chingam 5 on whose sunrise has the nakshatra for at least 2 h 24 min (or the day before, if the nakshatra started then and runs out within 2 h 24 min of this sunrise). |

- **The Chingam 5 cut-off for Onam is empirical.** In the official lists, a Thiruvonam on Chingam 1 to 4 was passed over twice (2013 and 2016), and one on Chingam 5 was taken twice (2002 and 2021). I know of no published rule that gives this cut-off, so treat it as fitted.
- **Onam fallbacks:** if no day in Chingam qualifies (1975 is the only year in the range), the day with the most nakshatra in daylight is used.

## Islamic holidays in India

Eid ul-Fitr (1 Shawwal), Eid al-Adha or Bakrid (10 Dhu al-Hijjah), Muharram or Ashura (10 Muharram) and Milad-un-Nabi (12 Rabi al-Awwal) come from date-holidays' own Islamic calendar rules, added to the India region by the generator.

## Warnings

- **`holiday_computed`:** every festival in this file carries it, because its date is computed, not taken from an official publication.
- **`holiday_moon_sighting`:** the Islamic holidays carry it, because the observed date can move by a day depending on the moon sighting.

Regional dates can differ from these. Tamil Nadu, Kerala, Bengal and Maharashtra each have their own reckoning for some festivals, and a few festivals (Makar Sankranti in particular) are kept on a conventional civil date in some states.

## Validation against the central government lists

The Department of Personnel & Training publishes the yearly list of holidays for Central Government offices in Delhi. Its gazetted and restricted dates for 2002–2004 and 2007–2027 (24 years, corrigenda applied) are in `tools/holidays/india_official.json`, each with its source document. `node tools/holidays/validate_india.mjs` compares them with the computed dates. The full result is in `tools/holidays/india_validation.json`.

| Festival | Agreement | Years that differ |
|---|---|---|
| Diwali | 24/24 | |
| Holi | 23/24 | 2004: official Mar 6, computed Mar 7 |
| Dussehra | 24/24 | |
| Janmashtami | 23/24 | 2002: official Aug 30, computed Aug 31 |
| Ganesh Chaturthi | 24/24 | |
| Maha Shivaratri | 24/24 | |
| Raksha Bandhan | 24/24 | |
| Ram Navami | 23/24 | 2008: official Apr 14, computed Apr 13 |
| Chaitra Navratri | 1/1 | |
| Navratri (Sharad) | no official dates | |
| Karva Chauth | 16/16 | |
| Makar Sankranti | 22/24 | 2004: official Jan 15, computed Jan 14. 2020: official Jan 15, computed Jan 14. |
| Pongal | 23/24 | 2004: official Jan 16, computed Jan 15 |
| Onam | 24/24 | |

That is 275 of 281 dated entries (97.9%).

**This is a fit, not a blind test.** I chose the day rules while looking at these lists. The first version, with textbook rules, agreed on 227 of 281. The changes after seeing the lists were:
- the bhadra and 3/4-day rule for Holika Dahan;
- the Shravana nakshatra test for Dussehra;
- udaya instead of nishita for Janmashtami;
- the 6-ghati test for Raksha Bandhan;
- the later-day preference for Ram Navami and Maha Shivaratri;
- the sunrise-to-sunrise day for Makar Sankranti;
- the last-fifth rule for Pongal;
- the Chingam 5 cut-off for Onam.

Each one, apart from the Onam cut-off, matches a rule given in traditional dharmashastra sources, but expect lower agreement on years outside 2002–2027.
