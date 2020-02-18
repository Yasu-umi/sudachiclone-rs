use std::fs::{create_dir_all, File};
use std::io::{Error as IOError, Write};
use std::path::Path;

pub fn write_sudachi_json<P: AsRef<Path>>(path: P) -> Result<(), IOError> {
  let path = path.as_ref();
  if !path.exists() {
    if let Some(p) = path.parent() {
      if !p.exists() {
        create_dir_all(p)?;
      }
    }
    File::create(path)?.write_all(SUDACHI_JSON.as_bytes())
  } else {
    Ok(())
  }
}

pub fn write_resources<P: AsRef<Path>>(dir: P) -> Result<(), IOError> {
  let dir = dir.as_ref();
  if !dir.join("char.def").exists() {
    File::create(dir.join("char.def"))?.write_all(CHAR_DEF.as_bytes())?;
  }
  if !dir.join("rewrite.def").exists() {
    File::create(dir.join("rewrite.def"))?.write_all(REWRITE_DEF.as_bytes())?;
  }
  if !dir.join("unk.def").exists() {
    File::create(dir.join("unk.def"))?.write_all(UNK_DEF.as_bytes())?;
  }
  Ok(())
}

const SUDACHI_JSON: &str = r#"
{
  "characterDefinitionFile" : "char.def",
  "inputTextPlugin" : [
      { "class" : "sudachipy.plugin.input_text.DefaultInputTextPlugin" },
      { "class" : "sudachipy.plugin.input_text.ProlongedSoundMarkInputTextPlugin",
        "prolongedSoundMarks": ["ー", "-", "⁓", "〜", "〰"],
        "replacementSymbol": "ー"}
  ],
  "oovProviderPlugin" : [
      { "class" : "sudachipy.plugin.oov.SimpleOovProviderPlugin",
        "oovPOS" : [ "補助記号", "一般", "*", "*", "*", "*" ],
        "leftId" : 5968,
        "rightId" : 5968,
        "cost" : 3857 }
  ],
  "pathRewritePlugin" : [
  ]
}
"#;

const CHAR_DEF: &str = r##"
#
#   Japanese charcter category map
#
#   $Id: char.def 9 2012-12-12 04:13:15Z togiso $;
#

###################################################################################
# 
#  CHARACTER CATEGORY DEFINITION
#
#  CATEGORY_NAME INVOKE GROUP LENGTH
#
#   - CATEGORY_NAME: Name of category. you have to define DEFAULT class.
#   - INVOKE: 1/0:   always invoke unknown word processing, evan when the word can be found in the lexicon
#   - GROUP:  1/0:   make a new word by grouping the same chracter category
#   - LENGTH: n:     1 to n length new words are added
#
DEFAULT         0 1 0  # DEFAULT is a mandatory category!
SPACE           0 1 0  
KANJI           0 0 2
SYMBOL          1 1 0
NUMERIC         1 1 0
ALPHA           1 1 0
HIRAGANA        0 1 2
KATAKANA        1 1 2
KANJINUMERIC    0 1 0  #change INVOKE 1->0
GREEK           1 1 0
CYRILLIC        1 1 0

###################################################################################
#
# CODE(UCS2) TO CATEGORY MAPPING
#

# SPACE
0x0020 SPACE  # DO NOT REMOVE THIS LINE, 0x0020 is reserved for SPACE
0x000D SPACE
0x0009 SPACE
0x000B SPACE
0x000A SPACE

# ASCII
0x0021..0x002F SYMBOL   #!"#$%&'()*+,-./
0x0030..0x0039 NUMERIC  #0-9
0x003A..0x0040 SYMBOL   #:;<=>?@
0x0041..0x005A ALPHA    #A-Z
0x005B..0x0060 SYMBOL   #[\]^_`
0x0061..0x007A ALPHA    #a-z
0x007B..0x007E SYMBOL   #{|}~

# Latin
0x00A1..0x00BF SYMBOL # Latin 1 #¡->¿
0x00C0..0x00D6 ALPHA  # Latin 1 #À->Ö
0x00D7         SYMBOL # Latin 1 #×
0x00D8..0x00F6 ALPHA  # Latin 1 #Ø->ö
0x00F7         SYMBOL # Latin 1 #÷
0x00F8..0x00FF ALPHA  # Latin 1 #ø->ÿ
0x0100..0x017F ALPHA  # Latin Extended A
0x0180..0x0236 ALPHA  # Latin Extended B
0x1E00..0x1EF9 ALPHA  # Latin Extended Additional

# CYRILLIC
0x0400..0x04F9 CYRILLIC #Ѐ->ӹ
0x0500..0x050F CYRILLIC # Cyrillic supplementary

# GREEK
0x0374..0x03FB GREEK # Greek and Coptic　#ʹ->ϻ

# HIRAGANA
0x3041..0x309F  HIRAGANA

# KATAKANA
#0x30A1..0x30FF  KATAKANA
0x30A1..0x30FA  KATAKANA
0x30FC..0x30FF  KATAKANA
0x31F0..0x31FF  KATAKANA  # Small KU .. Small RO
# 0x30FC          KATAKANA HIRAGANA  # ー
0x30A1          NOOOVBOW # Small A
0x30A3          NOOOVBOW
0x30A5          NOOOVBOW
0x30A7          NOOOVBOW
0x30A9          NOOOVBOW
0x30E3          NOOOVBOW
0x30E5          NOOOVBOW
0x30E7          NOOOVBOW
0x30EE          NOOOVBOW
0x30FB..0x30FE  NOOOVBOW

# Half KATAKANA
0xFF66..0xFF9D  KATAKANA
0xFF9E..0xFF9F  KATAKANA

# KANJI
0x2E80..0x2EF3  KANJI # CJK Raidcals Supplement
0x2F00..0x2FD5  KANJI
0x3005          KANJI NOOOVBOW
0x3007          KANJI
0x3400..0x4DB5  KANJI # CJK Unified Ideographs Extention
#0x4E00..0x9FA5  KANJI
0x4E00..0x9FFF  KANJI
0xF900..0xFA2D  KANJI
0xFA30..0xFA6A  KANJI


# KANJI-NUMERIC (一 二 三 四 五 六 七 八 九 十 百 千 万 億 兆)
0x4E00 KANJINUMERIC KANJI
0x4E8C KANJINUMERIC KANJI
0x4E09 KANJINUMERIC KANJI
0x56DB KANJINUMERIC KANJI
0x4E94 KANJINUMERIC KANJI
0x516D KANJINUMERIC KANJI
0x4E03 KANJINUMERIC KANJI
0x516B KANJINUMERIC KANJI
0x4E5D KANJINUMERIC KANJI
0x5341 KANJINUMERIC KANJI
0x767E KANJINUMERIC KANJI
0x5343 KANJINUMERIC KANJI
0x4E07 KANJINUMERIC KANJI
0x5104 KANJINUMERIC KANJI
0x5146 KANJINUMERIC KANJI

# ZENKAKU 
0xFF10..0xFF19 NUMERIC
0xFF21..0xFF3A ALPHA
0xFF41..0xFF5A ALPHA
0xFF01..0xFF0F SYMBOL   #！->／
0xFF1A..0xFF20 SYMBOL   #：->＠
0xFF3B..0xFF40 SYMBOL   #［->｀
0xFF5B..0xFF65 SYMBOL   #｛->･
0xFFE0..0xFFEF SYMBOL # HalfWidth and Full width Form

# OTHER SYMBOLS
0x2000..0x206F  SYMBOL # General Punctuation
0x2070..0x209F  NUMERIC # Superscripts and Subscripts
0x20A0..0x20CF  SYMBOL # Currency Symbols
0x20D0..0x20FF  SYMBOL # Combining Diaritical Marks for Symbols
0x2100..0x214F  SYMBOL # Letterlike Symbols
0x2150..0x218F  NUMERIC # Number forms
0x2100..0x214B  SYMBOL # Letterlike Symbols
0x2190..0x21FF  SYMBOL # Arrow
0x2200..0x22FF  SYMBOL # Mathematical Operators
0x2300..0x23FF  SYMBOL # Miscellaneuos Technical
0x2460..0x24FF  SYMBOL # Enclosed NUMERICs
0x2501..0x257F  SYMBOL # Box Drawing
0x2580..0x259F  SYMBOL # Block Elements
0x25A0..0x25FF  SYMBOL # Geometric Shapes
0x2600..0x26FE  SYMBOL # Miscellaneous Symbols
0x2700..0x27BF  SYMBOL # Dingbats
0x27F0..0x27FF  SYMBOL # Supplemental Arrows A
0x27C0..0x27EF  SYMBOL # Miscellaneous Mathematical Symbols-A
0x2800..0x28FF  SYMBOL # Braille Patterns
0x2900..0x297F  SYMBOL # Supplemental Arrows B
0x2B00..0x2BFF  SYMBOL # Miscellaneous Symbols and Arrows
0x2A00..0x2AFF  SYMBOL # Supplemental Mathematical Operators
0x3300..0x33FF  SYMBOL
0x3200..0x32FE  SYMBOL # ENclosed CJK Letters and Months
0x3000..0x303F  SYMBOL # CJK Symbol and Punctuation
0xFE30..0xFE4F  SYMBOL # CJK Compatibility Forms
0xFE50..0xFE6B  SYMBOL # Small Form Variants

# added 2006/3/13 
0x3007 SYMBOL KANJINUMERIC

# added 2018/11/30
0x309b..0x309c HIRAGANA KATAKANA # voiced/semi-voiced sound marks

# END OF TABLE
"##;

const REWRITE_DEF: &str = r#"
# ignore normalize list
#   ^{char}%n
Ⅰ
Ⅱ
Ⅲ
Ⅳ
Ⅴ
Ⅵ
Ⅶ
Ⅷ
Ⅸ
Ⅹ
Ⅺ
Ⅻ
Ⅼ
Ⅽ
Ⅾ
Ⅿ
ⅰ
ⅱ
ⅲ
ⅳ
ⅴ
ⅵ
ⅶ
ⅷ
ⅸ
ⅹ
ⅺ
ⅻ
ⅼ
ⅽ
ⅾ
ⅿ
⺀
⺁
⺂
⺃
⺄
⺅
⺆
⺇
⺈
⺉
⺊
⺋
⺌
⺍
⺎
⺏
⺐
⺑
⺒
⺓
⺔
⺕
⺖
⺗
⺘
⺙
⺛
⺜
⺝
⺞
⺟
⺠
⺡
⺢
⺣
⺤
⺥
⺦
⺧
⺨
⺩
⺪
⺫
⺬
⺭
⺮
⺯
⺰
⺱
⺲
⺳
⺴
⺵
⺶
⺷
⺸
⺹
⺺
⺻
⺼
⺽
⺾
⺿
⻀
⻁
⻂
⻃
⻄
⻅
⻆
⻇
⻈
⻉
⻊
⻋
⻌
⻍
⻎
⻏
⻐
⻑
⻒
⻓
⻔
⻕
⻖
⻗
⻘
⻙
⻚
⻛
⻜
⻝
⻞
⻟
⻠
⻡
⻢
⻣
⻤
⻥
⻦
⻧
⻨
⻩
⻪
⻫
⻬
⻭
⻮
⻯
⻰
⻱
⻲
⻳
⼀
⼁
⼂
⼃
⼄
⼅
⼆
⼇
⼈
⼉
⼊
⼋
⼌
⼍
⼎
⼏
⼐
⼑
⼒
⼓
⼔
⼕
⼖
⼗
⼘
⼙
⼚
⼛
⼜
⼝
⼞
⼟
⼠
⼡
⼢
⼣
⼤
⼥
⼦
⼧
⼨
⼩
⼪
⼫
⼬
⼭
⼮
⼯
⼰
⼱
⼲
⼳
⼴
⼵
⼶
⼷
⼸
⼹
⼺
⼻
⼼
⼽
⼾
⼿
⽀
⽁
⽂
⽃
⽄
⽅
⽆
⽇
⽈
⽉
⽊
⽋
⽌
⽍
⽎
⽏
⽐
⽑
⽒
⽓
⽔
⽕
⽖
⽗
⽘
⽙
⽚
⽛
⽜
⽝
⽞
⽟
⽠
⽡
⽢
⽣
⽤
⽥
⽦
⽧
⽨
⽩
⽪
⽫
⽬
⽭
⽮
⽯
⽰
⽱
⽲
⽳
⽴
⽵
⽶
⽷
⽸
⽹
⽺
⽻
⽼
⽽
⽾
⽿
⾀
⾁
⾂
⾃
⾄
⾅
⾆
⾇
⾈
⾉
⾊
⾋
⾌
⾍
⾎
⾏
⾐
⾑
⾒
⾓
⾔
⾕
⾖
⾗
⾘
⾙
⾚
⾛
⾜
⾝
⾞
⾟
⾠
⾡
⾢
⾣
⾤
⾥
⾦
⾧
⾨
⾩
⾪
⾫
⾬
⾭
⾮
⾯
⾰
⾱
⾲
⾳
⾴
⾵
⾶
⾷
⾸
⾹
⾺
⾻
⾼
⾽
⾾
⾿
⿀
⿁
⿂
⿃
⿄
⿅
⿆
⿇
⿈
⿉
⿊
⿋
⿌
⿍
⿎
⿏
⿐
⿑
⿒
⿓
⿔
⿕
豈
更
車
賈
滑
串
句
龜
龜
契
金
喇
奈
懶
癩
羅
蘿
螺
裸
邏
樂
洛
烙
珞
落
酪
駱
亂
卵
欄
爛
蘭
鸞
嵐
濫
藍
襤
拉
臘
蠟
廊
朗
浪
狼
郎
來
冷
勞
擄
櫓
爐
盧
老
蘆
虜
路
露
魯
鷺
碌
祿
綠
菉
錄
鹿
論
壟
弄
籠
聾
牢
磊
賂
雷
壘
屢
樓
淚
漏
累
縷
陋
勒
肋
凜
凌
稜
綾
菱
陵
讀
拏
樂
諾
丹
寧
怒
率
異
北
磻
便
復
不
泌
數
索
參
塞
省
葉
說
殺
辰
沈
拾
若
掠
略
亮
兩
凉
梁
糧
良
諒
量
勵
呂
女
廬
旅
濾
礪
閭
驪
麗
黎
力
曆
歷
轢
年
憐
戀
撚
漣
煉
璉
秊
練
聯
輦
蓮
連
鍊
列
劣
咽
烈
裂
說
廉
念
捻
殮
簾
獵
令
囹
寧
嶺
怜
玲
瑩
羚
聆
鈴
零
靈
領
例
禮
醴
隸
惡
了
僚
寮
尿
料
樂
燎
療
蓼
遼
龍
暈
阮
劉
杻
柳
流
溜
琉
留
硫
紐
類
六
戮
陸
倫
崙
淪
輪
律
慄
栗
率
隆
利
吏
履
易
李
梨
泥
理
痢
罹
裏
裡
里
離
匿
溺
吝
燐
璘
藺
隣
鱗
麟
林
淋
臨
立
笠
粒
狀
炙
識
什
茶
刺
切
度
拓
糖
宅
洞
暴
輻
行
降
見
廓
兀
嗀
﨎
﨏
塚
﨑
晴
﨓
﨔
凞
猪
益
礼
神
祥
福
靖
精
羽
﨟
蘒
﨡
諸
﨣
﨤
逸
都
﨧
﨨
﨩
飯
飼
館
鶴
郞
隷
侮
僧
免
勉
勤
卑
喝
嘆
器
塀
墨
層
屮
悔
慨
憎
懲
敏
既
暑
梅
海
渚
漢
煮
爫
琢
碑
社
祉
祈
祐
祖
祝
禍
禎
穀
突
節
練
縉
繁
署
者
臭
艹
艹
著
褐
視
謁
謹
賓
贈
辶
逸
難
響
頻
恵
𤋮
舘
並
况
全
侀
充
冀
勇
勺
喝
啕
喙
嗢
塚
墳
奄
奔
婢
嬨
廒
廙
彩
徭
惘
慎
愈
憎
慠
懲
戴
揄
搜
摒
敖
晴
朗
望
杖
歹
殺
流
滛
滋
漢
瀞
煮
瞧
爵
犯
猪
瑱
甆
画
瘝
瘟
益
盛
直
睊
着
磌
窱
節
类
絛
練
缾
者
荒
華
蝹
襁
覆
視
調
諸
請
謁
諾
諭
謹
變
贈
輸
遲
醙
鉶
陼
難
靖
韛
響
頋
頻
鬒
龜
𢡊
𢡄
𣏕
㮝
䀘
䀹
𥉉
𥳐
𧻓
齃
龎
゛
゜

# replace char list
#   ^{before}\s{after}%n
ｳﾞ	ヴ
ｶﾞ	ガ
ｷﾞ	ギ
ｸﾞ	グ
ｹﾞ	ゲ
ｺﾞ	ゴ
ｻﾞ	ザ
ｼﾞ	ジ
ｽﾞ	ズ
ｾﾞ	ゼ
ｿﾞ	ゾ
ﾀﾞ	ダ
ﾁﾞ	ヂ
ﾂﾞ	ヅ
ﾃﾞ	デ
ﾄﾞ	ド
ﾊﾞ	バ
ﾋﾞ	ビ
ﾌﾞ	ブ
ﾍﾞ	ベ
ﾎﾞ	ボ
ﾊﾟ	パ
ﾋﾟ	ピ
ﾌﾟ	プ
ﾍﾟ	ペ
ﾎﾟ	ポ
うﾞ	ゔ
かﾞ	が
きﾞ	ぎ
くﾞ	ぐ
けﾞ	げ
こﾞ	ご
さﾞ	ざ
しﾞ	じ
すﾞ	ず
せﾞ	ぜ
そﾞ	ぞ
たﾞ	だ
ちﾞ	ぢ
つﾞ	づ
てﾞ	で
とﾞ	ど
はﾞ	ば
ひﾞ	び
ふﾞ	ぶ
へﾞ	べ
ほﾞ	ぼ
はﾟ	ぱ
ひﾟ	ぴ
ふﾟ	ぷ
へﾟ	ぺ
ほﾟ	ぽ
ウﾞ	ヴ
カﾞ	ガ
キﾞ	ギ
クﾞ	グ
ケﾞ	ゲ
コﾞ	ゴ
サﾞ	ザ
シﾞ	ジ
スﾞ	ズ
セﾞ	ゼ
ソﾞ	ゾ
タﾞ	ダ
チﾞ	ヂ
ツﾞ	ヅ
テﾞ	デ
トﾞ	ド
ハﾞ	バ
ヒﾞ	ビ
フﾞ	ブ
ヘﾞ	ベ
ホﾞ	ボ
ハﾟ	パ
ヒﾟ	ピ
フﾟ	プ
ヘﾟ	ペ
ホﾟ	ポ
ゔ	ゔ
が	が
ぎ	ぎ
ぐ	ぐ
げ	げ
ご	ご
ざ	ざ
じ	じ
ず	ず
ぜ	ぜ
ぞ	ぞ
だ	だ
ぢ	ぢ
づ	づ
で	で
ど	ど
ば	ば
び	び
ぶ	ぶ
べ	べ
ぼ	ぼ
ぱ	ぱ
ぴ	ぴ
ぷ	ぷ
ぺ	ぺ
ぽ	ぽ
ヴ	ヴ
ガ	ガ
ギ	ギ
グ	グ
ゲ	ゲ
ゴ	ゴ
ザ	ザ
ジ	ジ
ズ	ズ
ゼ	ゼ
ゾ	ゾ
ダ	ダ
ヂ	ヂ
ヅ	ヅ
デ	デ
ド	ド
バ	バ
ビ	ビ
ブ	ブ
ベ	ベ
ボ	ボ
パ	パ
ピ	ピ
プ	プ
ペ	ペ
ポ	ポ
う゛	ゔ
か゛	が
き゛	ぎ
く゛	ぐ
け゛	げ
こ゛	ご
さ゛	ざ
し゛	じ
す゛	ず
せ゛	ぜ
そ゛	ぞ
た゛	だ
ち゛	ぢ
つ゛	づ
て゛	で
と゛	ど
は゛	ば
ひ゛	び
ふ゛	ぶ
へ゛	べ
ほ゛	ぼ
は゜	ぱ
ひ゜	ぴ
ふ゜	ぷ
へ゜	ぺ
ほ゜	ぽ
ウ゛	ヴ
カ゛	ガ
キ゛	ギ
ク゛	グ
ケ゛	ゲ
コ゛	ゴ
サ゛	ザ
シ゛	ジ
ス゛	ズ
セ゛	ゼ
ソ゛	ゾ
タ゛	ダ
チ゛	ヂ
ツ゛	ヅ
テ゛	デ
ト゛	ド
ハ゛	バ
ヒ゛	ビ
フ゛	ブ
ヘ゛	ベ
ホ゛	ボ
ハ゜	パ
ヒ゜	ピ
フ゜	プ
ヘ゜	ペ
ホ゜	ポ
"#;

const UNK_DEF: &str = r#"
DEFAULT,5968,5968,3857,補助記号,一般,*,*,*,*
SPACE,5966,5966,6056,空白,*,*,*,*,*
KANJI,5139,5139,14657,名詞,普通名詞,一般,*,*,*
KANJI,5129,5129,17308,名詞,普通名詞,サ変可能,*,*,*
KANJI,4785,4785,18181,名詞,固有名詞,一般,*,*,*
KANJI,4787,4787,18086,名詞,固有名詞,人名,一般,*,*
KANJI,4791,4791,19198,名詞,固有名詞,地名,一般,*,*
SYMBOL,5129,5129,17094,名詞,普通名詞,サ変可能,*,*,*
NUMERIC,4794,4794,12450,名詞,数詞,*,*,*,*
ALPHA,5139,5139,11633,名詞,普通名詞,一般,*,*,*
ALPHA,4785,4785,13620,名詞,固有名詞,一般,*,*,*
ALPHA,4787,4787,14228,名詞,固有名詞,人名,一般,*,*
ALPHA,4791,4791,15793,名詞,固有名詞,地名,一般,*,*
ALPHA,5687,5687,15246,感動詞,一般,*,*,*,*
HIRAGANA,5139,5139,16012,名詞,普通名詞,一般,*,*,*
HIRAGANA,5129,5129,20012,名詞,普通名詞,サ変可能,*,*,*
HIRAGANA,4785,4785,18282,名詞,固有名詞,一般,*,*,*
HIRAGANA,4787,4787,18269,名詞,固有名詞,人名,一般,*,*
HIRAGANA,4791,4791,20474,名詞,固有名詞,地名,一般,*,*
HIRAGANA,5687,5687,17786,感動詞,一般,*,*,*,*
KATAKANA,5139,5139,10980,名詞,普通名詞,一般,*,*,*
KATAKANA,5129,5129,14802,名詞,普通名詞,サ変可能,*,*,*
KATAKANA,4785,4785,13451,名詞,固有名詞,一般,*,*,*
KATAKANA,4787,4787,13759,名詞,固有名詞,人名,一般,*,*
KATAKANA,4791,4791,14554,名詞,固有名詞,地名,一般,*,*
KATAKANA,5687,5687,15272,感動詞,一般,*,*,*,*
KANJINUMERIC,4794,4794,14170,名詞,数詞,*,*,*,*
GREEK,5139,5139,11051,名詞,普通名詞,一般,*,*,*
GREEK,4785,4785,13353,名詞,固有名詞,一般,*,*,*
GREEK,4787,4787,13671,名詞,固有名詞,人名,一般,*,*
GREEK,4791,4791,14862,名詞,固有名詞,地名,一般,*,*
CYRILLIC,5139,5139,11140,名詞,普通名詞,一般,*,*,*
CYRILLIC,4785,4785,13174,名詞,固有名詞,一般,*,*,*
CYRILLIC,4787,4787,13495,名詞,固有名詞,人名,一般,*,*
CYRILLIC,4791,4791,14700,名詞,固有名詞,地名,一般,*,*
"#;
