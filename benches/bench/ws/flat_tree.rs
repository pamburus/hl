// std imports
use std::time::Duration;

// third-party imports
use const_str::concat as strcat;
use criterion::{criterion_group, BatchSize, BenchmarkId, Criterion, Throughput};

// workspace imports
use flat_tree::FlatTree;

// local imports
use super::{BencherExt, ND};

criterion_group!(benches, bench);

const GROUP: &str = strcat!(super::GROUP, ND, "flat-tree");

fn bench(c: &mut Criterion) {
    let mut c = c.benchmark_group(GROUP);
    c.warm_up_time(Duration::from_secs(3));
    c.measurement_time(Duration::from_secs(5));

    for n in [4, 64] {
        c.bench_function(BenchmarkId::new("push:x4", format!("r{}", n)), |b| {
            let setup = || {
                let mut container = FlatTree::<u64>::new();
                container.reserve(n);
                container
            };

            b.iter_batched_ref_fixed(
                setup,
                |container| {
                    container.push(42);
                    container.push(43);
                    container.push(44);
                    container.push(45);
                },
                BatchSize::NumIterations(4096),
            );
        });
    }

    c.throughput(Throughput::Elements(40 + 13));
    c.bench_function(BenchmarkId::new("build", "u64:p40b13d3:r64"), |b| {
        let setup = || {
            let mut container = FlatTree::<u64>::new();
            container.reserve(64);
            container
        };

        b.iter_batched_ref_fixed(
            setup,
            |container| {
                container.build(1, |node| {
                    node.push(2)
                        .push(3)
                        .build(4, |node| node.push(5).push(6).push(7).push(8).push(9))
                        .push(11)
                        .push(12)
                        .push(13)
                        .push(14)
                        .build(15, |node| node.push(16))
                        .build(18, |node| node.push(19))
                        .build(21, |node| node.push(22))
                        .build(24, |node| {
                            node.build(25, |node| node.push(26))
                                .build(28, |node| node.push(29).push(30).push(31).push(32).push(33))
                                .push(35)
                                .build(36, |node| node.push(37))
                                .build(39, |node| node.push(40).push(41))
                                .build(43, |node| node.push(44))
                        })
                        .push(47)
                        .push(48)
                        .build(49, |node| node.build(50, |node| node.push(51)).push(53))
                        .push(55)
                        .push(56)
                        .push(57)
                        .push(58)
                        .push(59)
                        .push(60)
                        .push(61)
                        .push(62)
                        .push(63)
                        .push(64)
                        .push(65)
                });
            },
            BatchSize::NumIterations(4096),
        );
    });

    c.throughput(Throughput::Elements(92 + 65));
    c.bench_function(BenchmarkId::new("build", "u64:p92b65d6:r160"), |b| {
        let setup = || {
            let mut container = FlatTree::<u64>::new();
            container.reserve(160);
            container
        };

        b.iter_batched_ref_fixed(
            setup,
            |container| {
                container.build(100, |node| {
                    node.build(200, |node| node.push(210).push(220))
                        .build(300, |node| node.push(310).push(320))
                        .build(400, |node| {
                            node.push(410).build(500, |node| {
                                node.build(600, |node| node.push(1).push(1))
                                    .build(700, |node| node.push(1).push(1))
                                    .build(800, |node| node.push(1).push(1))
                                    .build(900, |node| node.push(1).push(1))
                                    .build(1000, |node| node.push(1).push(1))
                            })
                        })
                        .build(1100, |node| node.push(1).push(1))
                        .build(1200, |node| node.push(1).push(1))
                        .build(1300, |node| node.push(1).push(1))
                        .build(1400, |node| node.push(1).push(1))
                        .build(1500, |node| {
                            node.push(1510)
                                .build(1520, |node| node.build(1600, |node| node.push(1610).push(1620)))
                        })
                        .build(1800, |node| {
                            node.push(1810)
                                .build(1820, |node| node.build(1900, |node| node.push(1910).push(1920)))
                        })
                        .build(2100, |node| {
                            node.push(2110)
                                .build(2120, |node| node.build(2200, |node| node.push(2210).push(2220)))
                        })
                        .build(2400, |node| {
                            node.push(2410).build(2420, |node| {
                                node.build(2500, |node| {
                                    node.push(2510)
                                        .build(2520, |node| node.build(2600, |node| node.push(2610).push(2620)))
                                })
                                .build(2800, |node| {
                                    node.push(2810).build(2820, |node| {
                                        node.build(2900, |node| node.push(2910).push(2920))
                                            .build(3000, |node| node.push(3010).push(3020))
                                            .build(3100, |node| node.push(3110).push(3120))
                                            .build(3200, |node| node.push(3210).push(3220))
                                            .build(3300, |node| node.push(3310).push(3320))
                                    })
                                })
                                .build(3500, |node| node.push(3510).push(3520))
                                .build(3600, |node| {
                                    node.push(3610)
                                        .build(3620, |node| node.build(3700, |node| node.push(3710).push(3720)))
                                })
                                .build(3900, |node| {
                                    node.push(3910).build(3920, |node| {
                                        node.build(4000, |node| node.push(4010).push(4020))
                                            .build(4100, |node| node.push(4110).push(4120))
                                    })
                                })
                                .build(4300, |node| {
                                    node.push(4310)
                                        .build(4320, |node| node.build(4400, |node| node.push(4410).push(4420)))
                                })
                            })
                        })
                        .build(4700, |node| node.push(4710).push(4720))
                        .build(4800, |node| node.push(4810).push(4820))
                        .build(4900, |node| {
                            node.push(4910).build(4920, |node| {
                                node.build(5000, |node| {
                                    node.push(5010)
                                        .build(5020, |node| node.build(5100, |node| node.push(5110).push(5120)))
                                })
                                .build(5300, |node| node.push(5310).push(5320))
                            })
                        })
                        .build(5500, |node| node.push(5510).push(5520))
                        .build(5600, |node| node.push(5610).push(5620))
                        .build(5700, |node| node.push(5710).push(5720))
                        .build(5800, |node| node.push(5810).push(5820))
                        .build(5900, |node| node.push(5910).push(5920))
                        .build(6000, |node| node.push(6010).push(6020))
                        .build(6100, |node| node.push(6110).push(6120))
                        .build(6200, |node| node.push(6210).push(6220))
                        .build(6300, |node| node.push(6310).push(6320))
                        .build(6400, |node| node.push(6410).push(6420))
                        .build(6500, |node| node.push(6510).push(6520))
                });
            },
            BatchSize::NumIterations(4096),
        );
    });

    c.throughput(Throughput::Elements(92 + 65));
    c.bench_function(BenchmarkId::new("build", "type+span:p92b65d6:r160"), |b| {
        type Span = std::ops::Range<usize>;

        #[allow(dead_code)]
        #[derive(Clone, Debug)]
        enum Node {
            Object(Span),
            Array(Span),
            Field(Span),
            String(Span),
            Number(Span),
            Boolean(Span),
            Null(Span),
        }

        let setup = || {
            let mut container = FlatTree::<Node>::new();
            container.reserve(160);
            container
        };

        b.iter_batched_ref_fixed(
            setup,
            |container| {
                container.build(Node::Object(0..1626), |node| {
                    node.build(Node::Field(1..40), |node| {
                        node.push(Node::String(1..13)).push(Node::String(14..40))
                    })
                    .build(Node::Field(41..55), |node| {
                        node.push(Node::String(41..51)).push(Node::String(52..55))
                    })
                    .build(Node::Field(56..235), |node| {
                        node.push(Node::String(56..63)).build(Node::Object(64..235), |node| {
                            node.build(Node::Field(65..118), |node| {
                                node.push(Node::String(65..79)).push(Node::String(80..118))
                            })
                            .build(Node::Field(119..154), |node| {
                                node.push(Node::String(119..129)).push(Node::String(130..154))
                            })
                            .build(Node::Field(155..198), |node| {
                                node.push(Node::String(155..159)).push(Node::String(160..198))
                            })
                            .build(Node::Field(199..216), |node| {
                                node.push(Node::String(199..205)).push(Node::String(206..216))
                            })
                            .build(Node::Field(217..234), |node| {
                                node.push(Node::String(217..226)).push(Node::String(227..234))
                            })
                        })
                    })
                    .build(Node::Field(236..285), |node| {
                        node.push(Node::String(236..247)).push(Node::String(248..285))
                    })
                    .build(Node::Field(286..305), |node| {
                        node.push(Node::String(286..294)).push(Node::String(295..305))
                    })
                    .build(Node::Field(306..336), |node| {
                        node.push(Node::String(306..314)).push(Node::String(315..336))
                    })
                    .build(Node::Field(337..356), |node| {
                        node.push(Node::String(337..346)).push(Node::String(347..356))
                    })
                    .build(Node::Field(357..382), |node| {
                        node.push(Node::String(357..362)).build(Node::Object(363..382), |node| {
                            node.build(Node::Field(364..381), |node| {
                                node.push(Node::String(364..373)).push(Node::String(374..381))
                            })
                        })
                    })
                    .build(Node::Field(383..423), |node| {
                        node.push(Node::String(383..389)).build(Node::Object(390..423), |node| {
                            node.build(Node::Field(391..422), |node| {
                                node.push(Node::String(391..397)).push(Node::String(398..422))
                            })
                        })
                    })
                    .build(Node::Field(424..449), |node| {
                        node.push(Node::String(424..431)).build(Node::Object(432..449), |node| {
                            node.build(Node::Field(433..448), |node| {
                                node.push(Node::String(433..439)).push(Node::String(440..448))
                            })
                        })
                    })
                    .build(Node::Field(450..889), |node| {
                        node.push(Node::String(450..462)).build(Node::Object(463..889), |node| {
                            node.build(Node::Field(464..498), |node| {
                                node.push(Node::String(464..475)).build(Node::Object(476..498), |node| {
                                    node.build(Node::Field(477..497), |node| {
                                        node.push(Node::String(477..483)).push(Node::String(484..497))
                                    })
                                })
                            })
                            .build(Node::Field(499..649), |node| {
                                node.push(Node::String(499..507)).build(Node::Object(508..649), |node| {
                                    node.build(Node::Field(509..528), |node| {
                                        node.push(Node::String(509..514)).push(Node::String(515..528))
                                    })
                                    .build(Node::Field(529..554), |node| {
                                        node.push(Node::String(529..540)).push(Node::String(541..554))
                                    })
                                    .build(Node::Field(555..587), |node| {
                                        node.push(Node::String(555..574)).push(Node::String(575..587))
                                    })
                                    .build(Node::Field(588..619), |node| {
                                        node.push(Node::String(588..597)).push(Node::String(598..619))
                                    })
                                    .build(Node::Field(620..648), |node| {
                                        node.push(Node::String(620..634)).push(Node::String(635..648))
                                    })
                                })
                            })
                            .build(Node::Field(650..671), |node| {
                                node.push(Node::String(650..661)).push(Node::String(662..671))
                            })
                            .build(Node::Field(672..716), |node| {
                                node.push(Node::String(672..678)).build(Node::Object(679..716), |node| {
                                    node.build(Node::Field(680..715), |node| {
                                        node.push(Node::String(680..686)).push(Node::String(687..715))
                                    })
                                })
                            })
                            .build(Node::Field(717..824), |node| {
                                node.push(Node::String(717..722)).build(Node::Object(723..824), |node| {
                                    node.build(Node::Field(724..778), |node| {
                                        node.push(Node::String(724..730)).push(Node::String(731..778))
                                    })
                                    .build(Node::Field(779..823), |node| {
                                        node.push(Node::String(779..784)).push(Node::String(785..823))
                                    })
                                })
                            })
                            .build(Node::Field(825..889), |node| {
                                node.push(Node::String(825..837)).build(Node::Object(838..889), |node| {
                                    node.build(Node::Field(839..888), |node| {
                                        node.push(Node::String(839..845)).push(Node::String(846..888))
                                    })
                                })
                            })
                        })
                    })
                    .build(Node::Field(890..904), |node| {
                        node.push(Node::String(890..897)).push(Node::String(898..904))
                    })
                    .build(Node::Field(905..944), |node| {
                        node.push(Node::String(905..916)).push(Node::String(917..944))
                    })
                    .build(Node::Field(945..1171), |node| {
                        node.push(Node::String(945..949))
                            .build(Node::Object(950..1171), |node| {
                                node.build(Node::Field(951..1152), |node| {
                                    node.push(Node::String(951..957))
                                        .build(Node::Object(958..1152), |node| {
                                            node.build(Node::Field(959..1151), |node| {
                                                node.push(Node::String(959..965)).push(Node::String(966..1151))
                                            })
                                        })
                                })
                                .build(Node::Field(1153..1171), |node| {
                                    node.push(Node::String(1153..1161)).push(Node::String(1162..1171))
                                })
                            })
                    })
                    .build(Node::Field(1172..1187), |node| {
                        node.push(Node::String(1172..1180)).push(Node::String(1181..1187))
                    })
                    .build(Node::Field(1188..1333), |node| {
                        node.push(Node::String(1188..1193)).push(Node::String(1194..1333))
                    })
                    .build(Node::Field(1334..1386), |node| {
                        node.push(Node::String(1334..1347)).push(Node::String(1348..1386))
                    })
                    .build(Node::Field(1387..1395), |node| {
                        node.push(Node::String(1387..1393)).push(Node::String(1394..1395))
                    })
                    .build(Node::Field(1396..1413), |node| {
                        node.push(Node::String(1396..1404)).push(Node::String(1405..1413))
                    })
                    .build(Node::Field(1414..1462), |node| {
                        node.push(Node::String(1414..1423)).push(Node::String(1424..1462))
                    })
                    .build(Node::Field(1463..1513), |node| {
                        node.push(Node::String(1463..1474)).push(Node::String(1475..1513))
                    })
                    .build(Node::Field(1514..1547), |node| {
                        node.push(Node::String(1514..1520)).push(Node::String(1521..1547))
                    })
                    .build(Node::Field(1548..1585), |node| {
                        node.push(Node::String(1548..1552)).push(Node::String(1553..1585))
                    })
                    .build(Node::Field(1586..1614), |node| {
                        node.push(Node::String(1586..1592)).push(Node::String(1593..1614))
                    })
                    .build(Node::Field(1615..1625), |node| {
                        node.push(Node::String(1615..1621)).push(Node::String(1622..1625))
                    })
                });
            },
            BatchSize::NumIterations(4096),
        );
    });

    c.finish();
}
