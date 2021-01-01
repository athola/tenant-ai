# real-estate-calculator
Real Estate Calculator built in Python/Django

## Project Intentions
To create a real estate calculator that will automate Real Estate Househack Analyses

Use example files as "mockups" -- turn them into code

Needs:
-Listing Price
-Number of Units (1-4)
-Square Footage
-Rehab Needed (None/Light/Medium/Heavy/Full) ---- Light = $15/sqft, Medium = $25/sq ft, Heavy = $35/sqft, Full = $50/sqft 
-Number of bedrooms
-Number of bathrooms
-Crime Level in Area (Low/Medium/High)
-Offer price @90%
-Total Rehab Estimate based on Square Footage and Rehab Needed
-Total Cost (=Offer Price + Rehab Estimate)
-Minimum Down Payment Required for Purchase (3.5% if any rehab necessary, 5% down if no rehab needed)
-Est Closing Costs (assume $2500)
-Total Cash Outlay (=Minimum Down Payment + Est Closing Costs)
-Conventional Loan Amount (=Total - Down Payment)
-Est Monthly Rent (using Rentometer, number of Bedrooms, Number of Bathrooms, and Address)
-Does it meet 1% rule (Y/N -- is Rent 1% or more of Purchase Price + Rehab)
-Est. Monthly Mortgage Payment (PITI + PMI) - calculate using MortgageCalculator.org, the Property Taxes from Realtor.com, an estimated annual insurance price of $1400
-Est Monthly OpEx (Mortgage Payment + Utilities ($140 lawn mowing/snow removal) + (Reserves = 25% of Monthly Rent))
-Cash on Cash Return on Investment = Est Monthly CashFlow * 12 / Total Cash Outlay
-Depreciation Tax Shield = Total Cost / 27.5 years * 22%
-Appreciation = Purchase Price * Neighborhood Rate of Appreciation (I typically assume 2%)
-Est Rent While HouseHacking = (N-1 Bedrooms) * average rent for a bedroom in the local area pulled from a website like Roomies.com
-Est Monthly Househacking CashFlow = HouseHacking Rent * 85% (assumes househacker will take care of Shoveling/LawnCare and Management while living in the househack
-Est Cash on Cash Return on Investment = Househacking Cashflow / Total Cash Outlay


Scrape Data for leads from following sites...
-Realtor.com
-HudHomestore.gov
-Homepath.com
-Crexi.com

...for the following areas
-Davenport, IA
-Bettendorf, IA
-Moline, IL
-East Moline, IL
-Rock Island, IL
-Peoria, IL
-Springfield, IL
