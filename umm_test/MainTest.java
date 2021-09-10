import org.junit.*;
import static org.junit.Assert.*;
import java.util.*;


import m.Main;

public class MainTest {
 
    private Collection collection;
    
    @BeforeClass
    public static void oneTimeSetUp() {
        // one-time initialization code        
    }
 
    @AfterClass
    public static void oneTimeTearDown() {
        // one-time cleanup code
    }
 
    @Before
    public void setUp() {
        collection = new ArrayList();
    }
    
    @After
    public void tearDown() {
        collection.clear();
    }
 
    @Test
    public void testEmptyCollection() {
        assertTrue(collection.isEmpty());
    }
    
    @Test
    public void testOneItemCollection() {
        collection.add("itemA");
        assertEquals(1, collection.size());
    }
}