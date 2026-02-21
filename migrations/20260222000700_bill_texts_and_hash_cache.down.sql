DROP TABLE IF EXISTS bill_texts;

ALTER TABLE bills
    DROP COLUMN IF EXISTS legiscan_change_hash,
    DROP COLUMN IF EXISTS legiscan_bill_id;
