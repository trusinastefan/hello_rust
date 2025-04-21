use server::password_hashing::{hash_password, verify_password};


#[tokio::test]
async fn test_hashing_and_verifying_same_strings() {
    let test_password = "Po1Po2Ca+tE3pE4tL".to_string();
    let test_password_hash = hash_password(&test_password).await.unwrap();
    let verify_result = verify_password(&test_password, &test_password_hash).await;
    assert!(verify_result.is_ok());
}

#[tokio::test]
async fn test_hashing_and_verifying_different_strings() {
    let test_password = "Po1Po2Ca+tE3pE4tL".to_string();
    let different_password = "aCoMpLeTeLyDiFfErEnTpAsSwOrD".to_string();
    let test_password_hash = hash_password(&test_password).await.unwrap();
    let verify_result = verify_password(&different_password, &test_password_hash).await;
    assert!(verify_result.is_err());
}
