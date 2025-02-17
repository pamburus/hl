pub(crate) mod query01 {
    pub const JSON: &str = r#""UPDATE \"apple\" SET \"seed\"='8c858361-5b73-442e-b84c-78482ed60ce1',\"planted_at\"=now() + timeout,\"importer\"='00d1cce2-c32e-4bb7-88da-474083fc2a1a',\"start_at\"=now() + repeat_interval,\"planted_at\"=now(),\"state\"='running',\"updated_at\"='2023-12-04 10:01:29.399' WHERE id IN (SELECT id FROM \"apple\" WHERE breed in ('red-delicious') AND distributor in ('magic-fruits','grand-provider') AND ((now() >= harvest_at AND (seed IS NULL OR (seed = 'b66134a4-c5c5-4adc-8c33-c8b7f780853b' AND importer != 'f86eb35d-33cd-499b-85cd-da175188e459'))) OR (now() >= planted_at)) ORDER BY \"updated_at\" LIMIT 4) AND ((now() >= harvest_at AND (seed IS NULL OR (seed = 'a3ecc839-0a32-4722-b4db-90c2ce8296a5' AND importer != '73a1fe4e-f4d1-4d09-99cb-9b07f2e32a96'))) OR (now() >= planted_at)) RETURNING *""#;

    pub const RAW: &str = r#"UPDATE "apple" SET "seed"='8c858361-5b73-442e-b84c-78482ed60ce1',"planted_at"=now() + timeout,"importer"='00d1cce2-c32e-4bb7-88da-474083fc2a1a',"start_at"=now() + repeat_interval,"planted_at"=now(),"state"='running',"updated_at"='2023-12-04 10:01:29.399' WHERE id IN (SELECT id FROM "apple" WHERE breed in ('red-delicious') AND distributor in ('magic-fruits','grand-provider') AND ((now() >= harvest_at AND (seed IS NULL OR (seed = 'b66134a4-c5c5-4adc-8c33-c8b7f780853b' AND importer != 'f86eb35d-33cd-499b-85cd-da175188e459'))) OR (now() >= planted_at)) ORDER BY "updated_at" LIMIT 4) AND ((now() >= harvest_at AND (seed IS NULL OR (seed = 'a3ecc839-0a32-4722-b4db-90c2ce8296a5' AND importer != '73a1fe4e-f4d1-4d09-99cb-9b07f2e32a96'))) OR (now() >= planted_at)) RETURNING *"#;
}

pub(crate) mod ipsum01 {
    pub const JSON: &str = r#""Lorem ipsum dolor sit amet, consectetur adipiscing elit. Ut euismod tincidunt mattis. Proin viverra elementum velit vel aliquam. Nullam in dolor risus. Donec tempus aliquet tellus, ac dignissim erat mattis aliquam. Maecenas interdum libero sed felis sodales, a lacinia sapien semper. Sed suscipit, est et auctor aliquam, purus erat porttitor metus, non tincidunt odio est a magna. Duis ac venenatis nulla, non aliquam justo. Vestibulum rhoncus odio ut est suscipit, varius sollicitudin metus consectetur. In feugiat justo at congue commodo. Fusce eros leo, varius nec neque et, pellentesque aliquam libero. Nam convallis eu leo at aliquam. Suspendisse vulputate lacinia nulla, sit amet malesuada quam malesuada at. Pellentesque neque odio, vehicula sed fringilla nec, dignissim vitae nulla ligula.""#;

    pub const RAW: &str = r#"Lorem ipsum dolor sit amet, consectetur adipiscing elit. Ut euismod tincidunt mattis. Proin viverra elementum velit vel aliquam. Nullam in dolor risus. Donec tempus aliquet tellus, ac dignissim erat mattis aliquam. Maecenas interdum libero sed felis sodales, a lacinia sapien semper. Sed suscipit, est et auctor aliquam, purus erat porttitor metus, non tincidunt odio est a magna. Duis ac venenatis nulla, non aliquam justo. Vestibulum rhoncus odio ut est suscipit, varius sollicitudin metus consectetur. In feugiat justo at congue commodo. Fusce eros leo, varius nec neque et, pellentesque aliquam libero. Nam convallis eu leo at aliquam. Suspendisse vulputate lacinia nulla, sit amet malesuada quam malesuada at. Pellentesque neque odio, vehicula sed fringilla nec, dignissim vitae nulla ligula."#;
}
