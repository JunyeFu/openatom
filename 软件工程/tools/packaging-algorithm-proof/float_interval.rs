// Isolated std-only numerical enclosure trial; no product geometry or dependency changes.
#[derive(Clone, Copy)]
struct Iv { lo: f64, hi: f64 }
impl Iv {
    fn new(lo: f64, hi: f64) -> Self {
        assert!(lo.is_finite() && hi.is_finite() && lo <= hi);
        Self { lo, hi }
    }
    fn point(x: f64) -> Self { Self::new(x,x) }
    fn add(self,b:Self)->Self { Self::new((self.lo+b.lo).next_down(),(self.hi+b.hi).next_up()) }
    fn neg(self)->Self { Self::new(-self.hi,-self.lo) }
    fn sub(self,b:Self)->Self { self.add(b.neg()) }
    fn mul(self,b:Self)->Self {
        let p=[self.lo*b.lo,self.lo*b.hi,self.hi*b.lo,self.hi*b.hi];
        let lo=p.iter().copied().fold(f64::INFINITY,f64::min).next_down();
        let hi=p.iter().copied().fold(f64::NEG_INFINITY,f64::max).next_up();
        Self::new(lo,hi)
    }
    fn div(self,b:Self)->Self {
        assert!(b.lo>0.0 || b.hi<0.0);
        self.mul(Self::new((1.0/b.hi).next_down(),(1.0/b.lo).next_up()))
    }
    fn pow(self,n:usize)->Self { (0..n).fold(Self::point(1.0),|v,_|v.mul(self)) }
}
fn fact(n:usize)->Iv { (1..=n).fold(Iv::point(1.0),|v,k|v.mul(Iv::point(k as f64))) }
fn trig(x:Iv,cosine:bool)->Iv {
    let m=x.lo.midpoint(x.hi);
    let radius=(m-x.lo).max(x.hi-m).next_up();
    let xm=Iv::point(m);let shift=if cosine {0} else {1};let mut value=Iv::point(0.0);
    for k in 0..18 {
        let term=xm.pow(2*k+shift).div(fact(2*k+shift));
        value=if k%2==0 {value.add(term)} else {value.sub(term)};
    }
    let degree=34+shift;
    let remainder=Iv::point(m.abs()).pow(degree+1).div(fact(degree+1)).hi;
    let error=Iv::point(remainder).add(Iv::point(radius)).hi;
    Iv::new(value.sub(Iv::point(error)).lo.max(-1.0),value.add(Iv::point(error)).hi.min(1.0))
}
fn pair(v:Iv)->String { format!("[{:.17e},{:.17e}]",v.lo,v.hi) }
fn main() {
    let arith=[
        ("add_decimal",Iv::point(0.1).add(Iv::point(0.2))),
        ("cancellation",Iv::point(1e16).add(Iv::point(1.0)).sub(Iv::point(1e16))),
        ("multiplication_scale",Iv::point(1e150).mul(Iv::point(1e-150))),
        ("one_third",Iv::point(1.0).div(Iv::point(3.0))),
    ];
    let angles=[(0.0,0.0),(1e-12,1e-12),(0.3,0.3),(1.5707963267948966,1.5707963267948966),(-3.0,-3.0),(0.0,1.0)];
    let mut rows:Vec<String>=arith.into_iter().map(|(name,v)|format!("{{\"id\":\"{name}\",\"interval\":{}}}",pair(v))).collect();
    for (i,(lo,hi)) in angles.iter().copied().enumerate() {
        for (name,cosine) in [("sin",false),("cos",true)] {
            rows.push(format!("{{\"id\":\"{name}_{i}\",\"input\":[{lo:.17e},{hi:.17e}],\"interval\":{}}}",pair(trig(Iv::new(lo,hi),cosine))));
        }
    }
    println!("[{}]",rows.join(","));
}
