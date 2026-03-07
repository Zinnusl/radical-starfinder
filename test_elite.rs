use crate::vocab::{resolve_compound_pinyin_step, CompoundPinyinStep, pinyin_syllables};

#[test]
fn test_elite_chain_progression() {
    let pinyin = "peng2you3";
    let syllables = pinyin_syllables(pinyin);
    println!("Syllables: {:?}", syllables);
    
    // Initial state
    let step0 = resolve_compound_pinyin_step(pinyin, 0, "peng2");
    println!("Step 0 with 'peng2': {:?}", step0);
    
    // After advancing
    let step1 = resolve_compound_pinyin_step(pinyin, 1, "you3");
    println!("Step 1 with 'you3': {:?}", step1);
    
    // What if progress exceeds total?
    let step2 = resolve_compound_pinyin_step(pinyin, 2, "you3");
    println!("Step 2 with 'you3': {:?}", step2);
    
    // What if progress is 0 but we type the second syllable?
    let step_wrong = resolve_compound_pinyin_step(pinyin, 0, "you3");
    println!("Step 0 with 'you3': {:?}", step_wrong);
}
